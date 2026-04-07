# vrt-check Procedure

Run VRT diff check if VRT tooling is detected.

## Procedure

1. Read [vrt-detection.md](vrt-detection.md) to detect VRT tooling.
2. If no VRT tooling detected → skip this phase (no action needed).
3. Run VRT command.
4. If diffs found → present diff images to user for review.
   - User approves → update baseline and commit.
   - User rejects → record in report only.
