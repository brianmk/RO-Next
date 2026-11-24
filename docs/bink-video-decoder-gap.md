# Bink video decoder gap

The 2019 client plays the in-game `.bik` movies (login screen, airship, dungeon
intros) through Bink Video. Korangar has **no Bink decoder** and this feature gap
did not add one (that would require a new proprietary-codec dependency).

What the client does instead:

1. `VideoLoader` only accepts the pre-transcoded AV1 `.ivf` side-car
   (`data\video\<name>.bik.ivf`) produced by the cache tool.
2. The cache tool (`sync_cache_archive` in
   `korangar/src/loaders/gamefile/cache.rs`) feeds each `.bik` through
   `ffmpeg -c:v libsvtav1 -f ivf`.
3. When FFmpeg is not installed, or the side-car is missing, the loader logs an
   explicit warning that Bink decoding is not implemented and uses the shipped
   placeholder texture. It never tries to decode `.bik` directly.

So a server/client installation that wants the movies must install FFmpeg and
run the cache sync once; otherwise the game is fully playable with the
placeholder movie frame. No code path silently pretends a `.bik` was decoded.
