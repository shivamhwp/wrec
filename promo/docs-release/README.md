# wrec docs release film

A 30-second Remotion promo for the new wrec docs. The cut uses `15.600–45.600`
from `Timeless (Instrumental).mp3`; the first major drop at source time `24.060`
lands at `00:08.460` (frame 254).

- `out/wrec-docs-release.mp4` is the 1920×1080 master.
- `out/wrec-docs-release-lakebed.mp4` is a compact hosted preview derived from
  the finished master because Lakebed capsules have no file store and a 2 MiB
  deployment-request ceiling.
- `lakebed/client/video-data.ts` is generated and embeds the hosted preview
  byte-for-byte as a data URL.

The audio excerpt and video outputs are intentionally ignored by Git.
