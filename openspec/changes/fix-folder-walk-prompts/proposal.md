# Proposal

## Why

On the owner's first install (2026-10-22), opening the New session window made macOS ask
whether Headquarter may access Apple Music and the photo library. The window searches
the home folder for project folders in the background, eight levels deep, and that
search read into `~/Music` and `~/Pictures`, where the Music and Photos libraries live.

## Expected Outcome

- Opening the New session window does not make macOS ask about Music, Photos or videos.
- Projects are still found everywhere else, including a project's own `Music` or
  `Pictures` folder.

## What Changes

- `hq-app` (`modal/folders.rs`): the folder search does not read `Music`, `Movies` and
  `Pictures` directly in the home folder, nor package folders (`.app`, `.photoslibrary`,
  `.musiclibrary`, `.tvlibrary`, `.imovielibrary`, `.bundle`) at any depth.

Decisions:
- **Skip, don't ask (owner, 2026-10-22).** These folders never hold projects. Rejected:
  searching only once the owner types a name (the search would still reach them).
- **Desktop, Documents and Downloads are still searched.** Projects can live there,
  so macOS may ask about them once; that question is a real need.

## Non-goals

- Browsing into those folders on purpose: typing `~/Music/` still lists it, and macOS
  may ask then.

## Capabilities

### Modified Capabilities
- `session-sidebar`: the folder search leaves media libraries alone.

## Impact

`crates/hq-app/src/modal/folders.rs` and its tests.

## Approval

Approved by the owner on 2026-10-22 ("No Music/Photos prompts").
