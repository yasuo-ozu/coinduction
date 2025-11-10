# General

- Use simple graph representation. Implement `ConstraintGraph` with Vec storage.
  - The graph stores constraints as Vec<TypeConstraint> and edges as Vec<(usize, usize)>
  - Node IDs are indices into the constraints vector
  - Edges represent dependency relationships between constraints
  - Root nodes represent the main constraint from impl block Self types
  - Strongly Connected Components (SCCs) using Tarjan's algorithm to detect cycles

# recurse.rs

- Implement `recurse` attribute macro that takes a module and arguments.
  - Ensure that the module has content.
- The arguments are comma-separated trait paths (not including path arguments), such as:
  - `MyTrait`
  - `path::to::MyTrait`
  - The path must be valid within the module.
  - Also accepts `coinduction = $path` form for crate path specification
- Iterate over the impl items in the module that implement traits given as
  arguments to `#[recurse(..)]`
  - Ensure that trait paths in impl blocks may have path arguments
- Assert that the `Self` position of the impl block consists of just one
  path segment (may have path arguments), not a double-colon-separated path.
- Create an empty working list that crosses all impl blocks.
- For each impl block, generate corresponding graphs as follows:
  - Expand the constraints in `Type: [for<..>] Trait [<..>]` form. Collect from both
    where predicate position and impl generics position.
    - Separate the predicates if there are multiple trait bounds. For example,
      separate `T: Trait1 + Trait2` into `T: Trait1` and `T: Trait2`.
  - Add a root node with the Self type constraint (Self type + impl trait).
  - For each extracted constraint, create a node and add an edge from root to that node.
  - Add the constraints to the working list if the trait is any of the arguments to
    `#[recurse(..)]`.
- Remove duplicate items from the working list.
- Pop the last item from the list.
  - Treat the trait path of the constraint as a macro path.
  - If no item exists, just emit a macro call to the `$crate::__finalize!` macro
- Finally, emit the module with the same name as the original module,
  additionally including:
  - A macro call to the macro path with the arguments:
    - The entire given module definition as tokens.
    - The working list of remaining constraints.
    - The target constraint in `Type: Trait` form.
    - Trait names given in the arguments of `#[recurse]`
    - Graphs
  - These arguments are common between macros. Use `MacroArgs` struct for consistency.
  - Additional constraints from `#[traitdef]` or `#[typedef]` are passed separately.


# traitdef.rs

- Provides `#[traitdef(..)]` attribute macro that accepts a trait definition
- Also accepts arguments (represented as TraitDefArgs struct):
  - `coinduction = $path` form argument to specify coinduction crate path
    (default: `::coinduction`)
  - Pattern-matching-like syntax to define type constraints for that pattern. Examples:
    - `($t1:ty, $t2:ty) => {$t1: MyTrait1, $t2: MyTrait2}`
    - `[$t1:ty, $e:expr] => {$t1: MyTrait1}`
- Generates a macro_rules that is exported to the macro namespace with the same namespace
  as the trait path.
  - The declarative macro accepts the target constraint and branch upon it.
  - If the type part of the target constraint matches some pattern, then pass
    the trait bounds to the `::coinduction::__internal!` proc macro.
  - Otherwise, treat the type part as macro name and call the macro.

# typedef.rs

- Takes an module and accepts comma-separated trait paths (with no path arguments)
  and coinduction crate path
  - The trait paths must valid within the module.
  as arguments.
- Provides a macro_rules exported to macro namespace with the same name as the type.
- List up the type definition (struct, enum, union, type alias) in the module.
- Get the impl block, which impls traits (given as arguments of the attribute macro)
  to any type (defined within the module).

# internal.rs

- Exports undocumented `__internal` functional macro.
- It receives:
  - The entire module passed from `#[recurse]`
  - The working list
  - The target constraint
  - Graphs corresponding to impl blocks in the base module
  - Additional constraints generated in `#[traitdef]` or `#[typedef]`
  - Trait names given in the arguments of `#[recurse]`
- For each graph, find a node which has target constraint as the weight. If found:
  - Add additional constraints as nodes in the graph and add edge from that node.
  - Replace type variables in additional constraints by matching against the target constraint.
  - Use simple type substitution to adapt constraint types to the current context.


# finalize.rs

- Exports `__finalize` functional macro, which is not documented and called internally.
- Takes the following arguments:
  - The entire original module
  - Graphs that correspond to the impl items.
- Modifies the original module and emits it. Modifications are as follows:
  - If the item is an impl block, find the corresponding Graph. If not found, skip that block.
  - Find strongly connected components (SCCs) using Tarjan's algorithm to detect cycles.
  - For each type constraint (including both where predicates and impl generics, separated),
    check if it is included in any cycle.
  - If a constraint is part of a cycle, remove it from the module definition.
  - Add leaf constraints (reachable from cycles but not part of cycles) to the where clause
    to maintain correctness while breaking the circular dependency.

