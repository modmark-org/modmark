use std::cmp::Ordering;
use std::collections::HashMap;
use std::ops::RangeFrom;

use bimap::BiBTreeMap;
use granular_id::{GranularId, UpperBounded};
use topological_sort::TopologicalSort;

use crate::element::GranId;
use crate::variables::{VarAccess, Variable};
use crate::{Context, Element};

type ScheduleId = usize;

// In short, each element is identified by its "GranId". We can use these Ids in our sorting, and
// then get the next ID to be evaluated. But when a parent node is evaluated, all its children
// is invalid and any returned IDs from its children (which is "new") may previously have been
// assigned to the previous children (which doesn't exist anymore). This means that we need to be
// able to "remove" IDs, but that isn't possible in TopologicalSort. For this reason, we assign
// new unique IDs to each GranId, of the type ScheduleId. We have an BiBTreeMap to keep track of
// this mapping.

pub struct Schedule {
    dag: TopologicalSort<ScheduleId>,
    dep_info: HashMap<Variable, Vec<(ScheduleId, VarAccess)>>,
    id_map: BiBTreeMap<GranId, ScheduleId>,
    id_iter: RangeFrom<ScheduleId>, // Assume this is infinite
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            dag: Default::default(),
            dep_info: Default::default(),
            id_map: Default::default(),
            id_iter: ScheduleId::MIN..,
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

        // schedule_id is the ID of the next node to be evaluated
        while let Some(schedule_id) = self.dag.pop() {
            // If this isn't in our BiBTreeMap it means that it doesn't exist anymore (it may once
            // have been a child of an element that is now evaluated). Then we pop the next ID.
            let Some((gran_id, _)) = self.id_map.remove_by_right(&schedule_id) else {
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
            // We collect all ScheduleIds that will now be invalid due to the pop operation
            let removed_ids = self
                .id_map
                .left_range(&gran_id..&end_range)
                .map(|(_gran, sched)| *sched)
                .collect::<Vec<_>>();
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

        // In this case, we couldn't pop one element, and this may occur either if there is a
        // cycle or if the DAG is empty. This will be checked externally.ﬁ
        None
    }

    pub fn add_element<T, U>(&mut self, element: &Element, ctx: &Context<T, U>) {
        // Element may be one of four kinds:
        // * Raw => We don't add it to the schedule
        // * Module => We add it to the schedule
        // * Parent => We add it and all its children to the schedule
        // * Compound => We add all its children to the schedule, but not the compound itself

        // Check if we have a compound element
        if let Element::Compound(children) = element {
            for child in children {
                self.add_element(child, &ctx);
            }
            return;
        }

        // For adding to the schedule, we need the name and GranId for that element
        // We could do this with a guard (let {name, id} = element else ...) but then rustfmt
        // doesn't format it for some reason
        let (name, this_id) = {
            match element {
                Element::Parent { name, id, .. } => (name, id),
                Element::Module { name, id, .. } => (name, id),
                _ => return,
            }
        };

        // Get new ScheduleId
        let this_schedule_id = self.id_iter.next().unwrap();

        // Insert this ScheduleId and put in the map
        self.dag.insert(this_schedule_id.clone());
        {
            let overwritten = self.id_map.insert(this_id.clone(), this_schedule_id);
            // Assert that the GranId isn't already in the map
            debug_assert!(matches!(overwritten, bimap::Overwritten::Neither));
        }

        // Look in the context to find info about which variables the element is interested in
        if let Some(variables) = &ctx.get_variable_accesses(name) {
            for (var, access) in variables {
                let mut deps = self.dep_info.remove(var).unwrap_or_default();

                // Update the dependency edges of the DAG
                for (other_schedule_id, other_access_type) in &deps {
                    let cmp = other_access_type.partial_cmp(access).unwrap();

                    match cmp {
                        Ordering::Less => {
                            self.dag.add_dependency(
                                other_schedule_id.clone(),
                                this_schedule_id.clone(),
                            );
                        }
                        Ordering::Greater => {
                            self.dag.add_dependency(
                                this_schedule_id.clone(),
                                other_schedule_id.clone(),
                            );
                        }

                        // Some variable types care about the order in which they are written to (like lists for instance)
                        Ordering::Equal if access.order_granular() => {
                            let other_id = self.id_map.get_by_right(other_schedule_id).unwrap();
                            match other_id.cmp(&this_id) {
                                Ordering::Less => {
                                    self.dag.add_dependency(
                                        other_schedule_id.clone(),
                                        this_schedule_id.clone(),
                                    );
                                }
                                Ordering::Greater => {
                                    self.dag.add_dependency(
                                        this_schedule_id.clone(),
                                        other_schedule_id.clone(),
                                    );
                                }
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }

                // Add this element to the deps map as well
                deps.push((this_schedule_id.clone(), *access));
                self.dep_info.insert(var.clone(), deps);
            }
        }

        // If the element is a parent, also add the children
        if let Element::Parent { children, .. } = element {
            children
                .iter()
                .for_each(|child| self.add_element(child, ctx));
        }
    }
}
