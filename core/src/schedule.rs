use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::RangeFrom;

use bimap::BiBTreeMap;
use granular_id::GranularId;
use num_traits::bounds::UpperBounded;
use topological_sort::TopologicalSort;

use crate::element::GranId;
use crate::variables::{VarAccess, Variable};
use crate::{Context, Element};

type TrashId = usize;

// In short, each element is identified by its "GranId". We can use these Ids in our sorting, and
// then get the next ID to be evaluated. But when a parent node is evaluated, all its children
// is invalid and any returned IDs from its children (which is "new") may previously have been
// assigned to the previous children (which doesn't exist anymore). This means that we need to be
// able to "remove" IDs, but that isn't possible in TopologicalSort. For this reason, we assign
// new unique IDs to each GranId, of the type TrashId. We have an BiBTreeMap to keep track if this
// mapping.

pub struct Schedule {
    dag: TopologicalSort<TrashId>,
    dep_info: HashMap<Variable, Vec<(TrashId, VarAccess)>>,
    id_map: BiBTreeMap<GranId, TrashId>,
    id_iter: RangeFrom<TrashId>, // Assume this is infinite
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            dag: Default::default(),
            dep_info: Default::default(),
            id_map: Default::default(),
            id_iter: TrashId::MIN..,
        }
    }
}

impl Schedule {
    pub fn is_empty(&self) -> bool {
        self.dag.is_empty()
    }

    pub fn pop(&mut self) -> Option<GranId> {
        // TODO LATER: use pop all and reorder them so modules of the same type are evaluated
        //  after each other, that way we can keep the same wasm module alive for better performance

        // trash_id is the ID of the next node to be evaluated
        while let Some(trash_id) = self.dag.pop() {
            // If this isn't in our BiBTreeMap it means that it doesn't exist anymore (it may once
            // have been a child of an element that is now evaluated). Then we pop the next ID.
            let Some((gran_id, _)) = self.id_map.remove_by_right(&trash_id) else {
                continue;
            };

            // We need to remove all children, and we do this by getting the next sibling and
            // extracting that range. If we, for example, want to evaluate the ID 0.1, we need to
            // remove all IDs 0.1.x, and we do that with the range 0.1..0.2. So, we find the next
            // sibling, or if none (we are root), take the max value of the type.
            let end_range = gran_id
                .next_siblings()
                .next()
                .unwrap_or(GranularId::max_value());
            // We collect all TrashIds that will now be invalid due to the pop operation
            let removed_ids = self
                .id_map
                .left_range(&gran_id..&end_range)
                .map(|(_gran, trash)| *trash)
                .collect::<Vec<_>>();
            println!("Removing {} ids", removed_ids.len());
            // For each ID that is removed, remove it from the ID map and from all dep infos
            removed_ids.into_iter().for_each(|id| {
                // By removing the ID from the map, it becomes a "tombstone id" and won't be
                // returned from further calls to this function
                self.id_map.remove_by_right(&id);
                // This may be optimised with other data structures
                self.dep_info.iter_mut().for_each(|(_k, v)| {
                    v.retain(|elem| elem.0 != id);
                });
            });
            return Some(gran_id);
        }

        if !self.dag.is_empty() {
            // FIXME: Don't panic here, return error
            panic!("Cycle");
        }
        None
    }

    pub fn add_element<T, U>(&mut self, element: &Element, ctx: &Context<T, U>) {
        if let Element::Compound(children) = element {
            for child in children {
                self.add_element(child, &ctx);
            }
            return;
        }

        let (
            Element::Parent{
                name, id: this_id, ..
            }
            |
            Element::Module {
                name, id: this_id, ..
            }
        ) = element else {
            // Handle raw and compound
            // NOTE: compound handled above
            return;
        };
        println!("Adding elem {name}; before: {} ids", self.id_map.len());

        // Get new TrashId
        let this_trashid = self.id_iter.next().unwrap();

        // Insert this TrashId and put in the map
        self.dag.insert(this_trashid.clone());
        {
            let overwritten = self.id_map.insert(this_id.clone(), this_trashid);
            // Assert that the GranId isn't already in the map
            debug_assert!(matches!(overwritten, bimap::Overwritten::Neither));
        }

        // Look in the context to find info about which variables the element is interested in
        if let Some(variables) = &ctx.get_variable_accesses(name) {
            for (var, access) in variables {
                let mut deps = self.dep_info.remove(var).unwrap_or_default();

                // Update the dependency edges of the DAG
                for (other_trashid, other_access_type) in &deps {
                    let cmp = other_access_type.partial_cmp(access).unwrap();

                    match cmp {
                        Ordering::Less => {
                            self.dag
                                .add_dependency(other_trashid.clone(), this_trashid.clone());
                        }
                        Ordering::Greater => {
                            self.dag
                                .add_dependency(this_trashid.clone(), other_trashid.clone());
                        }

                        // Some variable types care about the order in which they are written to (like lists for instance)
                        Ordering::Equal if access.order_granular() => {
                            let other_id = self.id_map.get_by_right(other_trashid).unwrap();
                            match other_id.cmp(&this_id) {
                                Ordering::Less => {
                                    self.dag.add_dependency(
                                        other_trashid.clone(),
                                        this_trashid.clone(),
                                    );
                                }
                                Ordering::Greater => {
                                    self.dag.add_dependency(
                                        this_trashid.clone(),
                                        other_trashid.clone(),
                                    );
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }

                // Add this element to the deps map as well
                deps.push((this_trashid.clone(), *access));
                self.dep_info.insert(var.clone(), deps);
            }
        }

        println!("Adding {name}; after: {} ids", self.id_map.len());
        // If the element is a parent, also add the children
        if let Element::Parent { children, .. } = element {
            children
                .iter()
                .for_each(|child| self.add_element(child, ctx));
        }
        println!("Adding {name}s children; after: {} ids", self.id_map.len());
    }
}
