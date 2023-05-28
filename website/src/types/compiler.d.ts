import * as Comlink from "comlink";

export type CompilationResult = {
  content: string;
  warnings: string[];
  errors: string[];
};

export type Compiler = Comlink.Remote<{
  loaded: boolean;
  init: () => Promise<void>;
  blank_context: () => Promise<void>;
  configure_from_source: (source: string) => Promise<boolean | null>;
  ast: (source: string) => Promise<null | string>;
  ast_debug: (source: string) => Promise<null | string>;
  json_output: (source: string) => Promise<null | string>;
  transpile: (source: string, format: string) => Promise<CompilationResult>;
  transpile_no_document: (source: string, format: string) => Promise<CompilationResult>;
  package_info: () => Promise<PackageInfo[] | null>;
  add_file: (path: string, bytes: Uint8Array) => Promise<void>;
  remove_file: (path: string) => Promise<void>;
  read_file: (path: string) => Promise<Uint8Array | null>;
  rename_entry: (from: string, to: string) => Promise<void>;
  get_file_list: (path: string) => Promise<any | null>;
  add_folder: (path: string) => Promise<void>;
  remove_folder: (path: string) => Promise<void>;
}>;

export type CompilationException =
  | {
      type: "compilationError";
      data: { message: string; raw: string }[];
    }
  | {
      type: "parsingError";
      data: { message: string; raw: string };
    }
  | { type: "noResult" };

export function handleException(expection: string): CompilationException {
  return JSON.parse(expection) as CompilationException;
}

export type Transform = {
  from: string;
  to: string[];
  description: string | null;
  arguments: ArgInfo[];
  variables: Record<string, VarInfo>;
  "unknown-content": boolean;
  "evaluate-before-children": boolean;
  type: string;
};

export type PackageInfo = {
  name: string;
  version: string;
  description: string;
  transforms: Transform[];
};

export type VarInfo = {
  type: "set" | "list" | "constant";
  access: string;
};

type ArgInfo = {
  name: string;
  default: string | null;
  description: string;
  type: string | string[];
};
