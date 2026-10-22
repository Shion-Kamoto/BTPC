#!/usr/bin/env python3
"""
Coupling drift checker + blast-radius lookup for BTPC.

Consumes scripts/coupling.json (machine-readable coupling index). Each concern
is one logical thing mirrored across N files that must stay in sync. The
checker greps the live source tree and FAILS when a mirror has drifted from
source_of_truth. First run is EXPECTED to fail on known-unfixed drift — that is
the point: surface staleness so readers stop trusting stale prose.

Usage:
  python3 scripts/coupling_check.py                  # check mode (exit 1 if any FAIL)
  python3 scripts/coupling_check.py --lookup <s>     # blast-radius: which concerns touch <s>?
  python3 scripts/coupling_check.py --json           # machine-readable result (CI)
  python3 scripts/coupling_check.py --root /path/to/repo

Expectations (per check):
  present      -> pattern must match at least once in the listed files
  absent       -> pattern must match NONE of the listed files (drift flag)
  present_all  -> pattern must match in EVERY listed file

Exit code: 0 = no drift, 1 = at least one check failed (or --lookup found nothing).

Why JSON not YAML: stdlib json has zero dependencies and is always present;
the index is still fully machine-readable and single-source.
"""
import argparse
import json
import os
import re
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def repo_root(explicit):
    if explicit:
        return os.path.abspath(explicit)
    # scripts/ lives at <repo>/scripts/, so root is the parent.
    return os.path.dirname(SCRIPT_DIR)


def load_index(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def search_file(pattern, relpath):
    """Return count of regex matches in one file (0 if unreadable)."""
    compiled = re.compile(pattern)
    abs_path = os.path.join(repo_root_value, relpath)
    try:
        with open(abs_path, encoding="utf-8", errors="replace") as fh:
            return len(compiled.findall(fh.read()))
    except OSError:
        return -1  # missing file


def evaluate(expect, counts):
    """counts: list of per-file match counts. -1 means file missing."""
    if expect == "present":
        return any(c > 0 for c in counts)
    if expect == "absent":
        return all(c <= 0 for c in counts)
    if expect == "present_all":
        return all(c > 0 for c in counts)
    return False


def run_checks(concerns):
    results = []
    for concern in concerns:
        for check in concern.get("checks", []):
            pattern = check["pattern"]
            expect = check["expect"]
            files = check.get("files", [])
            counts = [search_file(pattern, f) for f in files]
            ok = evaluate(expect, counts)
            results.append({
                "concern": concern["id"],
                "title": concern["title"],
                "severity": concern.get("severity", "medium"),
                "pattern": pattern,
                "expect": expect,
                "files": files,
                "counts": counts,
                "note": check.get("note", ""),
                "ok": ok,
            })
    return results


def lookup(symbol, concerns):
    hits = []
    for concern in concerns:
        touchpoints = []
        if symbol in concern.get("source_of_truth", ""):
            touchpoints.append(f"source_of_truth={concern['source_of_truth']}")
        for check in concern.get("checks", []):
            for f in check.get("files", []):
                if symbol in f:
                    touchpoints.append(f"check-file={f}")
        for note in concern.get("semantic_coupling", []):
            if symbol in note:
                touchpoints.append(f"semantic={note.strip()[:80]}")
        if touchpoints:
            hits.append((concern["id"], concern["title"], touchpoints))
    return hits


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=None, help="repo root (default: parent of scripts/)")
    ap.add_argument("--lookup", default=None, metavar="SYMBOL",
                    help="blast-radius: list concerns touching a file or symbol")
    ap.add_argument("--json", action="store_true", help="emit JSON result")
    args = ap.parse_args()

    global repo_root_value
    repo_root_value = repo_root(args.root)

    index_path = os.path.join(SCRIPT_DIR, "coupling.json")
    index = load_index(index_path)
    concerns = index["concerns"]

    if args.lookup:
        hits = lookup(args.lookup, concerns)
        if args.json:
            print(json.dumps({"query": args.lookup, "hits": [
                {"id": i, "title": t, "touchpoints": tp} for i, t, tp in hits]}, indent=2))
        else:
            if not hits:
                print(f"No coupling concerns touch '{args.lookup}'.")
                sys.exit(1)
            for cid, title, tp in hits:
                print(f"[{cid}] {title}")
                for t in tp:
                    print(f"    - {t}")
        return

    results = run_checks(concerns)
    failures = [r for r in results if not r["ok"]]
    semantic_only = [c for c in concerns if not c.get("checks")]

    if args.json:
        print(json.dumps({
            "total_checks": len(results),
            "passed": len(results) - len(failures),
            "failed": len(failures),
            "failures": [{
                "concern": r["concern"], "pattern": r["pattern"], "expect": r["expect"],
                "files": r["files"], "counts": r["counts"], "note": r["note"],
            } for r in failures],
            "semantic_only_concerns": [c["id"] for c in semantic_only],
        }, indent=2))
        sys.exit(1 if failures else 0)

    # Human-readable report
    print(f"Coupling drift check — {len(results)} mechanical checks across {len(concerns)} concerns\n")
    by_concern = {}
    for r in results:
        by_concern.setdefault(r["concern"], []).append(r)

    for concern in concerns:
        rs = by_concern.get(concern["id"], [])
        if not rs:
            print(f"[{concern['id']}] ({concern['severity']}) {concern['title']}")
            print(f"    source_of_truth: {concern['source_of_truth']}")
            print("    (semantic-only — no mechanical checks; see coupling.json)\n")
            continue
        print(f"[{concern['id']}] ({concern['severity']}) {concern['title']}")
        for r in rs:
            mark = "PASS" if r["ok"] else "FAIL"
            files_txt = ", ".join(r["files"]) or "(repo-wide)"
            print(f"    [{mark}] {r['expect']:<10} {r['pattern']!r}")
            print(f"           in {files_txt}  -> {r['counts']}")
            if r["note"] and not r["ok"]:
                print(f"           note: {r['note']}")
        print()

    print(f"Summary: {len(results) - len(failures)} passed, {len(failures)} failed "
          f"({len(semantic_only)} semantic-only concern(s))")
    if failures:
        print("\nFAILED checks are known-unfixed drift (expected on first run). "
              "Reconcile each mirror to source_of_truth, then this goes green.")
        sys.exit(1)


if __name__ == "__main__":
    main()
