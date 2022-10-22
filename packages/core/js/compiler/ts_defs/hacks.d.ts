// Most of the typings for `URL` (Deno's, TypeScript's, Node's, or `whatwg-url`) depend on good typings for `Symbol`. The typings for `Symbol` (whether you use Deno's or TypeScript's) depend on `IterableIterator`. Typings for `IterableIterator` depend on TypeScript's `lib.whatever.d.ts` definitions, which use `/// <reference path=*/>` references internally, which we don't implement, and also seem to introduce many conflicting type definitions when naively `cat`-ed together.
//
// To make a long story short, it's painful and manual to get good typings for `URL`, and the other Web globals. We're going to have to sort this out soon, but not at this very instant.
type URL = any;
type TextEncoder = any;
type TextDecoder = any;
