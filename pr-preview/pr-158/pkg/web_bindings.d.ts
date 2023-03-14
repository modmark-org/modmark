declare namespace wasm_bindgen {
	/* tslint:disable */
	/* eslint-disable */
	/**
	* @param {string} source
	* @returns {string}
	*/
	export function ast(source: string): string;
	/**
	* @param {string} source
	* @returns {string}
	*/
	export function ast_debug(source: string): string;
	/**
	* @param {string} source
	* @param {string} format
	* @returns {string}
	*/
	export function transpile(source: string, format: string): string;
	/**
	* @param {string} source
	* @returns {string}
	*/
	export function json_output(source: string): string;
	/**
	* @returns {string}
	*/
	export function package_info(): string;
	/**
	* @param {string} path
	* @returns {string}
	*/
	export function get_file_list(path: string): string;
	/**
	* @param {string} path
	* @param {Uint8Array} data
	*/
	export function add_file(path: string, data: Uint8Array): void;
	/**
	* @param {string} path
	*/
	export function add_folder(path: string): void;
	/**
	* @param {string} path
	* @param {string} new_path
	*/
	export function rename_entry(path: string, new_path: string): void;
	/**
	* @param {string} path
	*/
	export function remove_file(path: string): void;
	/**
	* @param {string} path
	*/
	export function remove_dir(path: string): void;
	/**
	* A struct representing an aborted instruction execution, with a message
	* indicating the cause.
	*/
	export class WasmerRuntimeError {
	  free(): void;
	/**
	* @returns {Symbol}
	*/
	  static __wbgd_downcast_token(): Symbol;
	}
	
}

declare type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

declare interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly ast: (a: number, b: number, c: number) => void;
  readonly ast_debug: (a: number, b: number, c: number) => void;
  readonly transpile: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly json_output: (a: number, b: number, c: number) => void;
  readonly package_info: (a: number) => void;
  readonly get_file_list: (a: number, b: number, c: number) => void;
  readonly add_file: (a: number, b: number, c: number, d: number) => void;
  readonly add_folder: (a: number, b: number) => void;
  readonly rename_entry: (a: number, b: number, c: number, d: number) => void;
  readonly remove_file: (a: number, b: number) => void;
  readonly remove_dir: (a: number, b: number) => void;
  readonly canonical_abi_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly canonical_abi_free: (a: number, b: number, c: number) => void;
  readonly __wbg_wasmerruntimeerror_free: (a: number) => void;
  readonly wasmerruntimeerror___wbgd_downcast_token: () => number;
  readonly __wbindgen_malloc: (a: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number) => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
  readonly __wbindgen_free: (a: number, b: number) => void;
  readonly __wbindgen_exn_store: (a: number) => void;
}

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
declare function wasm_bindgen (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
