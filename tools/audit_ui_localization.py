"""Audit source text and format tokens for the launcher localization.

Usage: python tools/audit_ui_localization.py --out analysis/ui-localization.json
No ROM data is read or copied. Rust render tests verify the widget wiring.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LITERAL = re.compile(r'"(?:\\.|[^"\\])*"', re.S)


def literals(source):
    source = "\n".join(line for line in source.splitlines() if not line.lstrip().startswith("//"))
    for match in LITERAL.finditer(source):
        try:
            yield json.loads(re.sub(r"\\\n\s*", "", match.group()))
        except ValueError:
            continue


def catalogue(path, marker="const ZH:", allow_identical_duplicates=False):
    source = path.read_text(encoding="utf-8").split(marker, 1)[1].split("\n];", 1)[0]
    values = list(literals(source))
    assert len(values) % 2 == 0, path
    pairs = list(zip(values[::2], values[1::2]))
    seen = {}
    for key, value in pairs:
        assert key not in seen or (allow_identical_duplicates and seen[key] == value), f"Duplicate key in {path}: {key}"
        seen[key] = value
    return dict(pairs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    launcher = ROOT / "crates/z2-launcher/src"
    shared = ROOT / "crates/z2-enh-ui/src"
    enh = catalogue(shared / "i18n.rs")
    rando = catalogue(launcher / "rando_ui/i18n.rs")
    main_ui = catalogue(launcher / "i18n.rs", allow_identical_duplicates=True)
    tables = {"enhancements": enh, "randomizer": rando}
    # Main-window keys take precedence at runtime.
    launch = rando | main_ui
    missing = []
    token_errors = []
    counts = {}
    files = list((launcher / "rando_ui").glob("*.rs"))
    files = [p for p in files if p.name != "i18n.rs"]
    sources = [(p, launch) for p in files]
    sources.append((shared / "lib.rs", enh))
    for path, table in sources:
        checked = 0
        for text in literals(path.read_text(encoding="utf-8").split("#[cfg(test)]")[0]):
            if not re.search("[A-Za-z]", text) or text.startswith(("rando-", "enh-")):
                continue
            if text in {"{z}x", "${i:02X}"} or re.fullmatch(r"\{[a-z_]+\}", text):
                continue
            checked += 1
            if text not in table:
                missing.append({"file": str(path.relative_to(ROOT)), "text": text})
        counts[str(path.relative_to(ROOT))] = checked
    flags = (ROOT / "crates/z2-rando/src/flags.rs").read_text(encoding="utf-8")
    enum_labels = set()
    for block in re.findall(r"flag_enum!\s*\{(.*?)\n\}", flags, re.S):
        enum_labels.update(re.findall(r'=>\s*"([^"\n]+)"', block))
    enum_labels.update(["Blue jar", "Red jar", "50 bag", "100 bag", "200 bag", "500 bag", "1-up", "Key"])
    for text in sorted(enum_labels):
        if re.search("[A-Za-z]", text) and text not in launch:
            missing.append({"file": "crates/z2-rando/src/flags.rs", "text": text})
    for name, table in tables.items():
        for original, translated in table.items():
            before = sorted(re.findall(r"\{[^{}]*\}", original))
            after = sorted(re.findall(r"\{[^{}]*\}", translated))
            if before != after:
                token_errors.append({"catalogue": name, "original": original, "translated": translated})
    corpus = "\n".join([*enh.values(), *rando.values(), *main_ui.values()])
    args.out.parent.mkdir(parents=True, exist_ok=True)
    corpus_file = args.out.with_name("ui-chinese-corpus.txt")
    corpus_file.write_text(corpus, encoding="utf-8")
    report = {
        "scope": "launcher enhancement/randomizer labels, help, enum labels and format tokens",
        "catalogue_counts": {name: len(table) for name, table in tables.items()},
        "checked_literals_by_file": counts,
        "enum_labels_checked": len(enum_labels),
        "missing": missing,
        "format_token_errors": token_errors,
        "corpus_sha256": hashlib.sha256(corpus.encode()).hexdigest(),
        "passed": not missing and not token_errors,
    }
    args.out.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))
    raise SystemExit(0 if report["passed"] else 1)


if __name__ == "__main__":
    main()
