Lunk is an event graph processing library, typically for chaining callbacks in user interfaces (think Rx/Reactive or Observable). I mostly use it with WASM, but I've used it with Gtk too.

Most web UI frameworks include their own event graph processing tools: Sycamore has signals, Dioxus and others have similar. This is a standalone tool, for use without a framework (i.e. with Gloo and other a la carte tools).

Compared to other tools, this library focuses on ease of use, flexibility, and composition with common language structures rather than on performance (although it appears to be decently fast anyway).

- You can represent a full graph

- Cycles are implicitly broken during execution

- Data has simple, lifetime-less types (`Prim<i32>`, `List<i32>`, etc) which are simple to pass around and store in structures

- Handles graph modifications during graph processing - new nodes will be processed after their dependencies

- Animation/easing property changes

This only handles synchronous graph processing. Asynchronous events are not handled.

Status: MVP

# Usage

## Basic usage

1. Create an `EventGraph` `eg`

2. Call `eg.event(|pc| { })` and do setup within it.

3. Within the event context, create values with `lunk::Prim::new()` and `lunk::List::new()`. Create links to process input changes with `lunk::link!()`.

4. When external events occur, call `eg.event` and modify data values using the associated methods.

5. At the end of `eg.event` link callbacks will be called for all newly created and downstream links of modified values.

An example:

```rust
fn main() {
    let eg = lunk::EventGraph::new();
    let (_input_a, _input_b, output, _link) = eg.event(|pc| {
        let input_a = lunk::Prim::new(0i32);
        let input_b = lunk::Prim::new(1f32);
        let output = lunk::Prim::new(0f32);
        let _link =
            lunk::link!(
                // Context
                (pc = pc),
                // Input values (primitives, lists)
                (input_a = input_a.clone(), input_b = input_b.clone()),
                // Output values (primitves, lists)
                (output = output.clone()),
                // Additional non-graph captures
                () {
                    output.set(ctx, input_a.get() as f32 * input_b.get() + 5.);
                }
            );
        input_a.set(pc, 46);
        return (input_a, input_b, output, _link);
    });
    assert_eq!(output.get(), 51.);
}
```

When `a` is modified, `b` will be updated to have the a's value plus 5.

See `link!` documentation for a detailed explanation. Links can be manually (without macros) defined but there's some boilerplate.

## Memory management and ownership

Links store strong references to their input and output values, but values store no references. You must keep all links alive for callbacks to happen.

A good way to keep them alive is to store them in the UI elements related to them. When the UI elements are removed the callbacks will no longer fire.

Keeping or accidentally dropping links can cause incorrect behavior - see the troubleshooting section below for suggestions.

## Animation

Primitive values can be interpolated over multiple steps.

1. Create an `Animator`

   Since you'll probably want to pass it around, making it a `Rc<RefCell<Animator>>` is a good solution.

2. Call `set_ease` on a primitive instead of `set`.

   `set_ease` requires an easing function - [ezing](https://github.com/michaelfairley/ezing) comes with a fairly complete set of easing functions:

   ```
   my_prim.set_ease(&mut animator, 44.3, 0.3, ezing::linear_inout);
   ```

   `set_ease` should be automatically implemented for any `Prim` where the value implements `Mult<f32>` and `Add` and `Sub` for its own type.

3. Call `update()` on the `Animator` regularly (at least until all animations finish).

   ```
   animator.update(&eg, delta_s);
   ```

   This step the animation (`delta_s` is seconds since the last update) and returns true if there are still in-progress animations.

You can also create your own custom animations by implementing `PrimAnimation` and calling `animator.start(MyPrimAnimation{...})` instead of `set_ease`.

### Idiomatic usage on the web

This was a bit tricky to work out, but I think the idiomatic way to use this in WASM currently with `request_animation_frame` (via Gloo):

```rust
let eg = lunk::EventGraph::new();
let anim = Rc::new(RefCell::new(Animator::new()));
anim.as_ref().borrow_mut().set_start_cb({
    let anim = anim.clone();
    let eg = eg.clone();
    let running = Rc::new(RefCell::new(None));

    fn one_more_frame(
        running: Rc<RefCell<Option<AnimationFrame>>>,
        eg: EventGraph,
        anim: Rc<RefCell<Animator>>,
    ) {
        *running.borrow_mut() = Some(request_animation_frame({
            let running = running.clone();
            move |delta| {
                if anim.as_ref().borrow_mut().update(&eg, delta) {
                    one_more_frame(running, eg, anim);
                } else {
                    *running.borrow_mut() = None;
                }
            }
        }));
    }

    move || {
        if running.borrow().is_some() {
            return;
        }
        one_more_frame(running.clone(), eg.clone(), anim.clone());
    }
});
```

Basically `request_animation_frame` needs to be called recursively to start the next frame, but the return value of each one needs to be stored until it's actually called.

To do this I made a shared `Option` for holding the return, which is kept alive inside the callback which is owned by `anim`.

## How the graph algorithm works

When an event occurs (wrapped by `eg.event`) the code modifies some values (primitives, lists). After this initial event handling finishes, the callback graph is executed.

The way this happens is as follows:

1. The dependency tree is collected by recursively walking outputs from all modified nodes/new links. This also identifies all nodes transitively "involved" in the event as well as leaves (nodes with no downstream nodes).

   In this step, "cycle" links (links that have an output that was an input earlier in the graph) are identified and filtered out. Note that cycles are broken even without this - this is an extra process to avoid links one step earlier (for the "better" feedback loop example again).

2. Using the leaves as roots, do a DFS through inputs. The DFS stops searching whenever it goes out of the "involved" node set or it encounters a node that had already started processing. Link callbacks are called as the DFS unwinds (i.e. after all their dependencies were processed).

3. If a link callback added new links to the graph, repeat (using the existing processed node set).

Re-entrant calls to `eg.event` (i.e. while `eg.event` is executing) are dropped.

### Preventing feedback loops

_Note: This depends on the execution environment, for instance JS doesn't trigger `change` event handlers from code-initiated changes during other event processing. However, GTK executes callbacks immediately when the value is changed and is thus affected._

A common scenario is: you have a textbox, something happens when the value changes, and also an external event (reset button, load button) can cause the value to change and this should be reflected in the textbox. Naively this would end up in an infinite feedback loop.

#### Naive implementation

A straightforward modeling of this scenario is:

- `Prim`, `my_value`

- `link!` (`l1`) from `my_value` with no output that sets the text of the texbox element

- An event handler on the textbox that does `ev.event(|pc| my_value.set(pc, el.value()))`

- An event handler on the reset button that does `ev.event(|pc| my_value.set(pc, original_value.clone()))`

When the reset button is pressed, this is what will execute:

1. The button press handler executes `eg.event` and does `my_value.set`

2. `l1` executes and modifies the textbox element text

3. The textbox change event handler is called, call to `eg.event` is ignored

4. Reached end of graph, `eg.event` ends

However, when the user types into the textbox this is executed:

1. The textbox change event handler executes `eg.event` and does `my_value.set`

2. `l1` executes and modifies the textbox element text

3. The textbox change event handler executes again, call to `eg.event` is ignored

4. Reached end of graph, `eg.event` ends

Per the above, you can see Lunk already implicitly prevents the infinite loop by linearizing the graph before execution.

However, it'll still try to change the textbox value - for other inputs this could mess with the cursor position or worse.

#### Better implementation

You can avoiding unnecessary work by doing this instead:

- `Prim`, `my_value`

- `Prim`, `textbox_value`

- `link!` (`l1`) from `my_value` to `textbox_value` that does `textbox_value.set(pc, my_value.borrow().clone())` and sets the textbox element text

- `link!` (`l2`) from `textbox_value` to `my_value` that does `my_value.set(pc, textbox_value.borrow().clone())`

- An event handler on the textbox that does `ev.event(|pc| textbox_value.set(pc, el.value()))`

- An event handler on the reset button that does `ev.event(|pc| my_value.set(pc, original_value))`

When the user types into the textbox, this will now happen:

1. The textbox change event handler executes `eg.event` and does `textbox_value.set` which marks `l2` as a processing root

2. `eg.event` walks `l2`. Then it walks `l1` and sees that the next link after `l1` is `l2` which was an ancestor on this path - so it must be a cycle. `l1` is skipped entirely, and `l2` is identified as a "leaf".

3. `eg.event` starts doing dependency-first traversal starting at `l2`. Since there's no dependencies, it executes `l2` and the callback sets `my_value`.

4. Reached end of graph, `eg.event` ends

No extra value modifications or element changes were made!

## Troubleshooting

- Mismatched types; expected fn pointer

  If you read down, it should say something like "closures can only be coerced to `fn` types if they do not capture any variables". This isn't about wrong types, it's about implicit captures.

  All captures in links created with `link!` need to be in one of the `()` at the start.

  In VS Code I need to click the link to see the full error before it shows which value is implicitly captured.

- Unreachable statement

  This occurs when you have an unconditional `return` in your callback. To support short ciruiting `Option` returns via `?`, `link!` adds a `return None` to the end of your function body. If you also return, this `return None` becomes unreachable, hence the alerts.

- My callback isn't firing

  Possible causes

  - The callback link was dropped or an input/output value captured by weak reference was dropped, or something earlier in the path to this node in the graph was dropped.

    Forward references are weak, so each dependent link object needs to be kept around as long as the callback is relevant.

  - You set the value but `PartialEq` determined it was equal to the current value

    If the value is the same, no updates will occur. This is to prevent unnecessary work when lots of changes are triggered by eliminating unmodified paths, but if you implemented `PartialEq` imprecisely it can prevent legitimate events from being handled.

  - You have your capture groups mixed up, and inputs are interpreted as something else (outputs, other captures).

    If you have the inputs in the wrong macro KV group they won't be acknowledged as a graph connection, so changes to dependencies won't trigger the callback.

  - You captured the output as a graph-unrelated value instead of using the 3rd `()` in the `link!` macro, so the graph processing doesn't recognize the changes or mis-orders the callback.

- My callback is firing and it shouldn't be

  Possible causes

  - The callback captures the item that owns it. For example, you did `link!` and captured an html element that the `link!` modifies, then store the `link!` handle in the html element itself. You should capture the html element by weak reference instead.

# Design decisions

## Separate links and data values

Some event graph implementations attach the callbacks directly to data values and connect data values directly, with no heterogenous "link" object.

In Lunk, by separating the value and link, the data types can be simple while the complexity is kept in the links. I think this makes ownership and lifetimes simpler - for instance, you won't accidentally keep listeners around just by storing a value.

This also supports many configurations with only a few functions/macros: value that's manually triggered not computed, a value that's computed from other values, and a computation that doesn't output a value.

## Requiring the user to specify the output type in the macro

The macro lacks the ability to detect the output type where it's needed.

I could have used template magic to infer the type, but this would have made manual (macro-less) link implementations need more boilerplate, so I decided against it. The types are fairly simple so I don't think it's a huge downside.

## Animations as a separate structure

In true a la carte philosophy, I figured some people might not want animations and it wasn't hard to make entirely separate.

If you use this, I think bundling the animator with the processing context shouldn't be too hard.

In case someone wants implements similar functionality as a 3rd party, the 1st party implementation only uses public interfaces so users should be able to freely choose a better solution.
