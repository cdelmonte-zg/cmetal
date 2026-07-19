# Hints and solutions

clings is built so you can get exactly as much help as you want, and no more.
Two mechanisms — progressive hints and earned solutions — sit on a spectrum
from "gentle nudge" to "here is the answer", and you control where on it you
land.

## Progressive hints

Every exercise ships a ladder of hints, ordered from the gentlest nudge to
almost the full answer. In watch mode, press `h` to reveal the first. Press `h`
again for the next. Hints **accumulate** on screen — you never lose an earlier
one — and you decide how far down the ladder to go.

From the CLI you can ask for a specific depth without entering watch mode:

```bash
clings hint pointers1              # first hint
clings hint pointers1 --level 2    # first two hints
```

The idea is deliberate: a good first hint reframes the problem
("`sizeof` on a pointer gives the pointer size, not the array size") rather than
handing you the fix. Reach for the next one only when the current one hasn't
unstuck you.

## Earned solutions

When an exercise passes, its official solution unlocks in `my_solutions/`. This
is the moment to compare: your fix against the reference, side by side. There is
often more than one correct answer, and seeing a second one is where a lot of
the learning happens.

Two design choices keep this honest:

- **Solutions unlock only after you solve the exercise.** You can't peek your
  way past a problem — the file simply isn't there until you've earned it.
- **Solutions are stored obfuscated in the repo** (`.c.enc` files), so casually
  browsing the repository on GitHub or in your editor never spoils an answer.
  clings decodes the one you earned into `my_solutions/` when you pass.

To re-open a solution you've already earned:

```bash
clings solution pointers2
```

## Starting over

To redo a single exercise — pristine file back, marked pending again,
everything else kept:

```bash
clings reset pointers1
```

For a clean slate — pristine exercises, no revealed solutions, progress
reset — use it without a name:

```bash
clings reset
```

That restores `my_exercises/` from the pristine `exercises/` and clears
`.clings-state.txt`. Your workspace (or checkout) is otherwise untouched.
