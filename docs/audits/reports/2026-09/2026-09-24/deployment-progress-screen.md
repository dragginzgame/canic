# Separate live deployment display from linear output

Date: 2026-09-24. Open batch: 0.110.41. Owner: `canic-cli`.

The supplied Toko transcript combined a live panel with permanent planning and
prerequisite messages. `Display::progress` classified prerequisites as immediate
linear output, observation callbacks cleared the panel to print diagnostics,
and resizing could switch from a live panel to scrolling milestones. Finishing
a receipt also left the last frame eligible for a later animation tick.

Interactive Fleet progress now owns one alternate screen for its lifetime.
Prerequisite transitions, placement collection and failed observations update
that screen. Terminal resizing clips the same display instead of changing
output modes. The screen omits the effects percentage-style bar and the duplicate
provisioning action line; effect counts remain accounting, not deployment percent.

Completion disables repainting, restores the ordinary terminal, then prints the
receipt location and summary. Final ensure reports/errors and generated desired
state output follow restoration. Ctrl-C and SIGTERM perform an atomic flag check
and async-signal-safe terminal write before default signal termination. A panic
restores the screen before the existing panic hook prints its diagnostic; drop
also restores after a poisoned display lock. No raw input mode or cursor hiding
is introduced. Receipt retention and deployment decisions remain unchanged.
Receipt write failures defer direct stderr output to the screen owner, remain
visible as an incomplete-receipt warning, and mark the final summary partial.

Redirected stderr, limited terminals, `NO_COLOR` and explicit JSON retain their
existing output modes. JSON and receipt schemas are unchanged. The live screen
does not stream extra planning summaries or milestones into the ordinary terminal.

## Qualification

Evidence: `.tmp/deployment-screen-20260924/`.

- 29 focused progress, receipt and catalog tests pass. Subprocess cases exercise
  both prerequisite checkpoints in the supplied transcript, observation success
  and failure, ensure/generation finish, late callbacks, animation ticks, SIGINT,
  SIGTERM and panic. Assertions cover terminal protocol boundaries and signal
  outcomes. Read-only file handles induce receipt write/finalization failures;
  the retained failure flag changes without a direct stderr write.
- Plain/JSON callback regressions and existing timing/retry receipt tests pass.
  Broken-write cleanup returns the typed IO error and attempts terminal restore.
- A real pseudo-terminal replay resizes from normal dimensions to 24 columns /
  12 rows and back. Its retained transcript has one alternate-screen entry, one
  exit, and 29 in-screen redraws, with following output outside the screen.
  This is an observed fixture result, not a required redraw count or timing gate.
- Strict scoped CLI lint, formatting, layering, document semantics and whitespace
  checks accompany the change. No deployment, IC call or broad suite is required
  to exercise this presentation boundary.

This correction extends the existing .41 deployment batch and both changelog
views. All work remains uncommitted. The separate compilation/finalization
feasibility investigation was paused to handle this direct operator feedback;
no production compilation identity change is included here.
