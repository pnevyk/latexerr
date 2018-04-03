# Contributing Guide

## New rules

Implementing a new rule is fairly simple. See [rules.rs](https://github.com/pnevyk/latexerr/blob/master/src/rules.rs)
for inspiration. The process usually consists of:

* Adding the variant into `LogItemType` enum with little documentation.
* Specifying if the rule corresponds to error or warning in `impl` block of `LogItemType`.
* Creating new empty struct represtning the rule.
* Implementing `Rule` trait for the struct.
* Implementing `Display` code for `LogItem` enum for the rule variant.
* Adding the struct reference to `rules` method of `LogItem`.
