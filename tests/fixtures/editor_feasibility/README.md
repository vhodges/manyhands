# S1 pure-helper counterexamples

These are independently written synthetic originals and expectations, not
candidate-generated goldens. No actual Editor or native input runs in S1.
`cases.toml` records exact UTF-8 body/edit offsets. Local `.gitattributes`
disables newline conversion; CRLF and missing final newline are intentional.

- `crlf-math.md`: immutable CRLF metadata with unknown nested YAML and mixed
  prose/math. `crlf-math-observed.md` is a **source-derived, simulated** body
  normalization counterexample; `crlf-math-lf.md` independently models newline
  loss. Neither is a passing preservation expectation or actual readback.
- `unicode-table.md`: combining/non-BMP/bidi text, table, unsupported directive
  and HTML. `unicode-table-edited.md` changes only `one` to `ONE`; header and
  every other byte must remain identical.
- `no-final-newline.md` and `math-observed.md`: the reviewed mixed `$$` source
  counterexample. Length equality alone is insufficient (both are 29 bytes).
- `empty-body.md`: protected header and zero-length body.

Large sources repeat the documented pattern, cut at the last UTF-8 boundary
not exceeding the requested size, then pad with ASCII spaces. Fixed BLAKE3
hashes in `cases.toml` identify exactly 102400 and 262144 bytes. They were
established from the deterministic original generator, never from editor output.
This tests helper length/hash retention, not native responsiveness or usability.

## Evidence contract for S2

Construct one `Draft` per document, using `Original::new` on the unchanged repo
text. Leading `---` frontmatter through its closing delimiter is immutable;
unclosed headers error rather than being repaired. Give only `body()` to the
editor. Keep the same editor/draft across mode/selector changes. Initial
`observe(generation, body)` records load readback; subsequent readbacks expose
current-vs-loaded edits separately from load-vs-original differences. An undo
to the loaded normalized bytes does not make original preservation pass.
Missing readback has no status and is not clean. Normalized or edited drafts
cannot be replaced; `replace_clean` requires explicit action and current
matching generation, increments generation and clears readbacks. Old callbacks
must be rejected. This is not a production observation/draft store.

Validate byte ranges with `validate_range` before calling candidate APIs; they
must not silently clamp/snap. `expected_edit` produces an independent comparator,
not an editing model. Compare actual candidate bytes against originals/goldens,
not against a normalized expectation disguised as the original.

`Readback` carries generation, exact body and provenance. Only readback obtained
from the real editor may be labeled `ActualEditorReadback`; pure model tests
use `SimulatedHelper`. `assess(None)` is `MissingNotTested`, never a pass. A byte
difference can be experimentally tolerated and recorded, but
`actual_exact_preservation` remains false. Capture's full candidate is the
**host-retained header plus editor body**, not proof that the editor preserved
metadata itself; `compare_full` also checks independently supplied full source.

`CaptureRun::create(run_id)` is an explicit capture action, always creates a
fresh directory under ignored `target/editor-feasibility/<run_id>/` and rejects
existing paths, traversal and symlink directories. Each `capture` creates a
fresh named child and create-new files only. It verifies current draft readback,
records original/header/body/full BLAKE3 hashes, generation, load/edit/dirty
status, evidence provenance and caller-provided build/action identity in
`manifest.toml`. Missing evidence creates no candidate files. No writes target
repo docs. Partial I/O failures return errors and retain partial evidence;
there is no overwrite, automatic persistence, home/cache access or recovery.
This local single-writer seam is not a hostile-filesystem sandbox. S2 owns
native resource policy, real action traces and accurate running build identity.

The focused test keeps captures with `s1-helper-*` labels as ignored local
simulated evidence. No live services, keys, clipboard, network or native host
are exercised. Full final Rust/CLI/native gates belong to S2's amended tree.
