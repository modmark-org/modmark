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

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly ast: (a: number, b: number, c: number) => void;
  readonly ast_debug: (a: number, b: number, c: number) => void;
  readonly transpile: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly json_output: (a: number, b: number, c: number) => void;
  readonly package_info: (a: number) => void;
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

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {SyncInitInput} module
*
* @returns {InitOutput}
*/
export function initSync(module: SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {InitInput | Promise<InitInput>} module_or_path
*
* @returns {Promise<InitOutput>}
*/
export default function init (module_or_path?: InitInput | Promise<InitInput>): Promise<InitOutput>;
