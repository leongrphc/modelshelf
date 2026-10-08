# Download behavior

## Files, ownership and identity

A job records a repository, immutable commit, selected relative paths, sizes and provider SHA-256 hashes
where present. A unique job folder is created below an authorized storage directory. Partial data and
checkpoint metadata use reserved `.modelshelf-*` names. The repository's nested structure is preserved.
Unknown file formats remain downloadable. ModelShelf never runs downloaded code.

## Recovery

The scheduler persists state in SQLite. After an interrupted session, active jobs become Paused rather than
silently assuming completion. Resume checks the actual partial-file length and a SHA-256 checkpoint of its prefix.
Trailing bytes beyond a durable checkpoint can be discarded. A missing/invalid checkpoint restarts the file.

Continuation sends `Range` and `If-Range`. It requires a strong unchanged ETag and an exact `Content-Range`
matching the pinned size and requested offset. A fresh signed URL alone proves nothing about file identity.
A full `200` response causes truncation/restart; reused-byte reporting stays zero. An invalid `206` is rejected
without appending data. Native Xet chunk reuse is not claimed; the HTTPS compatibility transport is used.

Before publication, the actual size must match. If provider SHA-256 metadata exists, the hash must also match.
A durable publication checkpoint permits recovery after publication but before a SQLite commit. Hardlink publication
is atomic and refuses to replace an existing final path. The temporary link is then removed. Job/library completion
is committed in one SQLite transaction. Completed files are rechecked before the job is finalized.

## Controls

The scheduler runs one file per active job and up to the configured 1–3 concurrent jobs/files.
Queued jobs can move earlier. Pause/cancel are cooperative at progress checkpoints, so network stalls can delay
acknowledgement until the read timeout. Closing the app stops transfer activity; resume is explicit after restart.
Cancel retains partial data for retry and never removes unrelated or external files.

Retries use bounded exponential delay; 401/403/404 and integrity failures are not retried indefinitely.
Unknown network/filesystem failures are surfaced in the job history. Progress updates are throttled and ETA
is displayed only when transfer speed can be calculated.

## Verification labels

- **Verified:** a trusted provider SHA-256 matches the completed file and its expected size.
- **Unverified:** no trusted cryptographic hash was available, even if size and local hash were checked.
- **Corrupted:** a trusted hash or expected size did not match.
- **Missing:** an indexed local file is no longer present.

Manual integrity checks stream file contents; they can take time for large models. They never change file contents.

## Automated evidence

Local mock tests interrupt a deterministic transfer, persist progress, reopen the database/service, request a range,
verify reuse, verify SHA-256, and check the completed library entry. Other tests cover ignored ranges and hash mismatch.
The opt-in live test searches Hugging Face and downloads only the public GPT-2 `config.json`:

```powershell
cargo test -p modelshelf-download --test live_hub -- --ignored --nocapture
```

No multi-gigabyte weight download is required for automated tests. Real-world long-running acceptance tests remain
part of release qualification.
