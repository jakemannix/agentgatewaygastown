/**
 * Type-safe path builder for JSONPath expressions
 *
 * Usage:
 *   const $ = createPathBuilder<MyType>();
 *   $.data.items[0].name  // -> "$.data.items[0].name"
 */
declare const PATH_SYMBOL: unique symbol;
/**
 * Path builder type - allows type-safe access to nested properties
 */
export type PathBuilder<T> = T extends object ? {
    [K in keyof T]: PathBuilder<T[K]>;
} & {
    toString(): string;
    [PATH_SYMBOL]: string;
} : {
    toString(): string;
    [PATH_SYMBOL]: string;
};
/**
 * Create a type-safe path builder
 *
 * @example
 * interface Data {
 *   user: { name: string; email: string };
 *   items: { id: number; title: string }[];
 * }
 *
 * const $ = createPathBuilder<Data>();
 * const namePath = $.user.name;  // PathBuilder with path "$.user.name"
 * String(namePath)  // "$.user.name"
 */
export declare function createPathBuilder<T>(): PathBuilder<T>;
/**
 * Extract the path string from a path builder
 */
export declare function getPath<T>(builder: PathBuilder<T>): string;
/**
 * Simple $ helper that returns a path string
 * Use when you don't need type safety
 */
export declare const $: {
    /**
     * Create a path expression
     * @example $._("data.items[0].name") -> "$.data.items[0].name"
     */
    _: (path: string) => string;
    /**
     * Root path
     */
    root: string;
};
/**
 * Shorthand for creating path expressions
 * @example path("data.items") -> "$.data.items"
 */
export declare function path(expr: string): string;
/**
 * Array index access
 * @example index("items", 0) -> "$.items[0]"
 */
export declare function index(basePath: string, idx: number): string;
/**
 * Wildcard array access
 * @example wildcard("items") -> "$.items[*]"
 */
export declare function wildcard(basePath: string): string;
export {};
//# sourceMappingURL=path-builder.d.ts.map