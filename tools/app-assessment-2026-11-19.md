# App assessment - 2026-11-19

Performed directly in the main agent session. No subagents or delegation.

## Result

Core browser flows worked against the running debug/mock bridge. Four user-visible issues were
reproduced and fixed locally. This is a scoped functional and layout assessment, not certification
of every native integration or WCAG compliance.

## Fixed

- **Profile clipping at large text sizes:** the fixed sidebar left too little room for history.
  At 743 CSS px, the history region was 286 px wide but its content required 442 px. The profile
  now stacks its summary above history at narrow widths; search controls and filters wrap.
- **Builds clipping at narrow widths:** the content region was 296 px wide with 463 px of content
  before selecting a build. The champion picker now sits above the build at narrow widths;
  rune columns and build sections adapt to their container. The picker retains its own scroll.
- **Settings Appearance overflow:** a 367 px section required 382 px, clipping its control.
  The theme preview and description now wrap while preserving the preview's intended width.
- **Elapsed-day grammar:** history displayed “1 days ago.” Day labels now use the UI locale's
  relative-time plural rules, including French, Spanish and Russian. Regression coverage checks
  the hour/day boundary and Russian one/few/many forms.

## Verification

- Opened Profile, Live game, Meta, Tier lists, Builds, Hextech guide, Patch notes and Settings.
- Observed both champ-select and in-game states, including champion-specific augment recommendations.
- Searched for Ahri and loaded her build, including Electrocute and item recommendations.
- Selected the empty Solo/Duo history filter, restored All, expanded a match, switched to damage
  and healing, and verified the ranking contained ten players.
- Inspected narrow-layout screenshots. Profile main content and match rows had no horizontal
  overflow at 743, 901, 1000, 1040, 1041 and 1320 CSS px after layout settled.
- Checked narrow Builds with loaded Ahri/Skarner builds and Settings Appearance without horizontal
  overflow. Russian Profile loaded and displayed correct day grammar; this was not an exhaustive
  Russian UI review.
- `npm run check`: no errors or warnings. `npm test`: 78 passed.
- `cargo test --lib --quiet`: 83 passed, 2 ignored.
- `npm run build`: succeeded. Main JS chunk is approximately 663 KB minified / 215 KB gzip.
- Impeccable's source detector returned no findings for the changed view files. `git diff --check`
  passed. Source checks do not prove rendered contrast or complete accessibility.

## Limits and follow-up

- Browser previews use the actual frontend and debug bridge. Their native WebView zoom command is
  a no-op, so 743 x 486 CSS px approximates the 1040 x 680 minimum window at 140% zoom.
  Verify the final layouts in the native WebView before release.
- The running bridge reported version 1.0.18 while source/package version was 1.0.19. Frontend
  edits were live through Vite; backend tests used current source. The old debug process was not rebuilt.
- Real-client imports, global hotkeys, card OCR, installers, updates and fullscreen behavior were
  not exercised. No rune/item writes or external publication were performed.
- The production bundle warning remains. Route-level code splitting is a separate performance task;
  no startup-performance claim is made here.
- Browser console showed a missing preview favicon and intermittent Data Dragon image HTTP 503s.
  These did not prevent the checked flows. Asset-network reliability remains a dependency.
- Russian expanded score rows showed approximately 3 px of local overflow in some rows; no full
  localization sweep was performed.
- T3 preview screenshot capture stopped producing frames. Playwright supplied the final visual
  checks. Screenshots and prior debug logs remain in the local, untracked `tools/.audit/` directory.

These fixes are included in the 1.0.20 release preparation. No PR was created.
