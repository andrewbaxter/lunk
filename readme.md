Lunk is an event graph processing library.

The expected use case is user interfaces, where user events (clicks, typing) can set off cascades of other changes and computation. Specifically, WASM/web user interfaces (since it's single threaded) but this could also reasonably be used with other single-threaded UI libraries like Gtk.

Most web UI frameworks include their own event graph processing tools: Sycamore has signals, Dioxus and others have similar. This is a standalone tool, for use without a framework (i.e. with Gloo and other a la carte tools).

Compared to other tools, this library focuses on ease of use, flexibility, and composition with common language structures rather than on performance (although it appears to be pretty fast anyway).

- You can represent a full graph, including cycles
- Data has simple, lifetime-less types (`Prim<i32>`, `List<i32>`, etc) which makes it simple to pass around and store in structures
- Handles graph modifications during graph processing - new nodes will be processed after their dependencies
- Animation/easing property changes

This only handles synchronous graph processing at the moment. Asynchronous events are not handled.

Status: MVP

# Usage

## Basic usage

1. Create an `EventGraph` `eg`
2. Call `eg.event(|pc| { })` to do setup.
3. Within the event context, create values with `lunk::Prim::new()` and `lunk::List::new()`. Create links to process input changes with `lunk::link!()`.
4. When user input happens, call `eg.event` and modify data values using the associated methods.

An example:

```rust
fn main() {
    let eg = lunk::EventGraph::new();
    let (_input_a, _input_b, output, _link) = eg.event(|pc| {
        let input_a = lunk::Prim::new(pc, 0i32);
        let input_b = lunk::Prim::new(pc, 1f32);
        let output = lunk::Prim::new(pc, 0f32);
        let _link =
            lunk::link!(
                (ctx = pc),
                (input_a = input_a.clone(), input_b = input_b.clone()),
                (output = output.clone()),
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

## Ownership

Values have weak references to dependent links and no references to the links that write to them. This means that there are no rc-leaking cycles in the graph structure itself.

Links must be kept alive for the duration you want them to trigger. If a link is dropped, it will stop activating during events.

## Animation

To animate primitive values

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

Aside from `set_ease`, you can create your own custom animations by implementing `PrimAnimation` and calling `animator.start(MyPrimAnimation{...})`.

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

## Troubleshooting

### Mismatched types; expected fn pointer

If you read down, it should say something like "closures can only be coerced to `fn` types if they do not capture any variables". This isn't about wrong types, it's about implicit captures.

All captures in links created with `link!` need to be in one of the `()` at the start.

In VS Code I need to click the link to see the full error before it shows which value is implicitly captured.

### Unreachable statement

This occurs when you have an unconditional `return` in your callback. To support short ciruiting `Option` returns via `?`, `link!` adds a `return None` to the end of your function body. If you also return, this `return None` becomes unreachable, hence the alerts.

### My callback isn't firing

Possible causes

- The callback link was dropped or an input/output value captured by weak reference was dropped, or something earlier in the path to this node in the graph was dropped.

  Forward references are weak, so each dependent link object needs to be kept around as long as the callback is relevant.

- You set the value but `PartialEq` determined it was equal to the current value

  If the value is the same, no updates will occur. This is to prevent unnecessary work when lots of changes are triggered by eliminating unmodified paths, but if you implemented `PartialEq` imprecisely it can prevent legitimate events from being handled.

- You have your capture groups mixed up, and inputs are interpreted as something else (outputs, other captures).

  If you have the inputs in the wrong macro KV group they won't be acknowledged as a graph connection, so changes to dependencies won't trigger the callback.

### My callback is firing (leaked)

Possible causes

- The callback captures the item that owns it.  For example, you did `link!` and captured an html element that the `link!` modifies.  You should change the capture to a weak reference.

# Why flexibility over performance

The main gains from these libraries come from helping you avoid costly work. The more flexible, the more work it'll help you avoid.

For work it doesn't avoid, it's still fast: it's written in Rust, the time spent doing graph processing is miniscule compared to FFI calls, styling, layout, rendering in a web environment.

Despite performance not being a focus, it's actually very fast! I tried integrating it into <https://github.com/krausest/js-framework-benchmark> and got good performance: 746ms vs 796ms for Sycamore (!)

(I don't quite believe this is faster than Sycamore:)

- There may be some randomness in the benchmark, it's a browser after all
- Some of the difference may be due to non-event-graph things, like element creation (direct `rooting` manipulation vs Sycamore's JSX-like system)

# Design decisions

## Separate links and data values

You typically pass around data so other systems can attach their own listeners. If a value is an output of a graph computation, it needs to include references to all the inputs the computation needs, which in general means you need a new type for each computation. By keeping the data separate from the link, the data types can be simple while the complexity is kept in the links.

This also supports many configurations with only a few functions/macros: value that's manually triggered not computed, a value that's computed from other values, and a computation that doesn't output a value.

## Requiring the user to specify the output type in the macro

The macro lacks the ability to detect the output type where it's needed.

I could have used template magic to infer the type, but this would have made manual (macro-less) link implementations need more boilerplate, so I decided against it. The types are fairly simple so I don't think it's a huge downside.

## Animations as a separate structure

In true a la carte philosophy, I figured some people might not want animations and it wasn't hard to make entirely separate.

I think bundling the animator with the processing context shouldn't be too hard.

In case there are other similar extensions, having a solution that allows external extension is important (maybe this won't happen though, then I may go ahead and integrate it).
