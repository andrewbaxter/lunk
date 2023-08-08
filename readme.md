- Processes event graph rooted at modified nodes
- Handles loops, no infinite repeats
- Handles graph modifications during graph walking - new nodes will be activated in dependency order.

Why

Optimizations and sugar magic in other libraries prevent normal program composition (ownership, lifetimes). They suggest idiomatic usage removes this requirement, but the idiomatic usage isn't well defined and there are still situations where contorting usage to match suggested idioms leads to much messier code.

Event processing is a graph, so the library should consider everything this implies (loops, graph changes, etc).

While this is performant, optimization is a goal. Event graph processing is fundamentally a way to avoid doing unnecessary work. For performance critical parts of the application like graphics, where everything is being recomputed continuously, there's no significant benefit to using an event graph (i.e. escape the graph and do all processing in one indivisible function) so optimizing for this sort of use case is not a goal.

Speed

Design decisions

- Separate links and values: Many possible combinations of incoming + outgoing links, including no links - would have needed lots of types and methods to suppor them all.
- Requiring dest type in link macro: could be inferred, but moves boilerplate into macro which makes it harder for manual implementers
