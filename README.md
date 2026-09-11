# Jatabag CLI

This repo provides cli commands for working with Jatabag trees.

[Jatabag](https://jatabag.com) gives a useful [Focus View](https://jatabag.com/focus), along with affective [traits](https://jatabag.com/traits/), so you can see the next thing you should be working on (without laboriously maintaining hand-edited todo lists).

## Install

Download binaries from the [releases page](https://github.com/jatabag/jata-todo-cli/releases). Or:

```sh
cargo install --path .
```

## Save an existing tree

Supply the link to one or more trees, and they will be stored locally.

```sh
jata https://jatabag.com/tree/2e4e6863-aaaa-aaaa-aaaa-aa6da5ba51eb
```

## Read a tree

```sh
jata tree                     # the whole tree
jata tree --ids               # with ids
jata tree --tag errands       # only tagged
jata focus                    # only what is actionable
jata focus --flat --sort stale-most
```

If just 1 tree is stored locally, then it assumes that tree, otherwise a picker will trigger.

## Write to a tree

```sh
jata new Buy milk                               # selects first or starts a picker
jata new Ship it --notes "the new release"
jata new Groceries --category
jata delete
jata delete --purge                             # purge subtrees (careful)
```