/**
 * Type-safe path builder for JSONPath expressions
 *
 * Usage:
 *   const $ = createPathBuilder<MyType>();
 *   $.data.items[0].name  // -> "$.data.items[0].name"
 */

// Proxy handler for building JSONPath strings
const pathProxyHandler: ProxyHandler<{ path: string }> = {
  get(target, prop) {
    if (prop === 'toString' || prop === Symbol.toPrimitive) {
      return () => target.path;
    }
    if (prop === '__path__') {
      return target.path;
    }
    if (typeof prop === 'string') {
      const newPath = `${target.path}.${prop}`;
      return new Proxy({ path: newPath }, pathProxyHandler);
    }
    return undefined;
  },
};

// Symbol to extract path from proxy
const PATH_SYMBOL = Symbol('path');

/**
 * Path builder type - allows type-safe access to nested properties
 */
export type PathBuilder<T> = T extends object
  ? {
      [K in keyof T]: PathBuilder<T[K]>;
    } & {
      toString(): string;
      [PATH_SYMBOL]: string;
    }
  : {
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
export function createPathBuilder<T>(): PathBuilder<T> {
  return new Proxy({ path: '$' }, pathProxyHandler) as unknown as PathBuilder<T>;
}

/**
 * Extract the path string from a path builder
 */
export function getPath<T>(builder: PathBuilder<T>): string {
  // Force conversion to string
  return String(builder);
}

/**
 * Simple $ helper that returns a path string
 * Use when you don't need type safety
 */
export const $ = {
  /**
   * Create a path expression
   * @example $._("data.items[0].name") -> "$.data.items[0].name"
   */
  _: (path: string): string => (path ? `$.${path}` : '$'),

  /**
   * Root path
   */
  root: '$',
};

/**
 * Shorthand for creating path expressions
 * @example path("data.items") -> "$.data.items"
 */
export function path(expr: string): string {
  if (expr.startsWith('$')) {
    return expr;
  }
  return expr ? `$.${expr}` : '$';
}

/**
 * Array index access
 * @example index("items", 0) -> "$.items[0]"
 */
export function index(basePath: string, idx: number): string {
  const base = path(basePath);
  return `${base}[${idx}]`;
}

/**
 * Wildcard array access
 * @example wildcard("items") -> "$.items[*]"
 */
export function wildcard(basePath: string): string {
  const base = path(basePath);
  return `${base}[*]`;
}

