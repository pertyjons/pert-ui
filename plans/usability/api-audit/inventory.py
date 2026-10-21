#!/usr/bin/env python3
"""Builds the frozen inputs of the API consistency audit.

Slice 4 of plans/developer-usability.md; the rubric beside this script says
what the inputs are for. For every package and configuration that
api/generate.sh lists, the script

1. builds the rustdoc JSON with generate.sh's command and flags and keeps a
   copy in target/api-audit/<package>/<configuration>/rustdoc.json;
2. records its provenance;
3. checks the copy against the committed api/ listing: cargo-public-api must
   reproduce the listing byte for byte, and the public type and module paths
   the script finds must be the listing's. It stops on a mismatch.

It then writes the inputs to plans/usability/api-audit/inputs/:

- provenance.json;
- mechanical candidate lists (candidates/*.txt), derived from the JSON;
- semantic review lists (semantic/*.txt), one per package: every public
  type with its inherent and trait-declared methods and an empty role
  column for the auditor. The script never assigns a role.

Usage: plans/usability/api-audit/inventory.py [--skip-build] [--out DIR]
--skip-build reuses the copies already in target/api-audit/. Needs the
pinned toolchain and the cargo-public-api version generate.sh names.
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "plans/usability/api-audit/inputs"
WORK = ROOT / "target/api-audit"
GENERATE = ROOT / "api/generate.sh"

NUMERIC = {
    "u8", "u16", "u32", "u64", "u128", "usize",
    "i8", "i16", "i32", "i64", "i128", "isize", "f32", "f64",
}
# Parameter type names that mark a constructor as binding state (rubric C3).
STATE_NAMES = re.compile(
    r"(StateOwner|Runtime|Signal|Selection|Controller|Host|Session|Binding|State"
    r"|^NavigationHistory|^ToastQueue|^SeriesSource|^Parameter)$")
CONVERSION_TRAITS = {"From", "TryFrom", "AsRef", "AsMut", "FromStr", "Into", "TryInto"}
AUTO_TRAITS = {"Send", "Sync", "Unpin", "UnwindSafe", "RefUnwindSafe", "Freeze"}


def run(cmd, **kw):
    return subprocess.run(cmd, cwd=ROOT, check=True, text=True, capture_output=True, **kw)


def configurations():
    """(package, configuration, feature arguments) as generate.sh lists them."""
    text = GENERATE.read_text()
    target = re.search(r"^TARGET=(\S+)$", text, re.M).group(1)
    tool = re.search(r"^TOOL_VERSION=(\S+)$", text, re.M).group(1)
    configs = []
    for m in re.finditer(r"^listing (\S+) (\S+)(.*)$", text, re.M):
        configs.append((m.group(1), m.group(2), m.group(3).split()))
    return target, tool, configs


def build(package, config, features, target):
    """Builds one configuration's JSON as generate.sh does and keeps a copy."""
    env = {
        "CARGO_ENCODED_RUSTFLAGS": "",
        "CARGO_ENCODED_RUSTDOCFLAGS": "",
        "RUSTC_BOOTSTRAP": "1",
    }
    full_env = dict(os.environ, **env)
    build_dir = WORK / "build"
    subprocess.run(
        ["cargo", "rustdoc", "--locked", "--quiet", "-p", package, "--target", target,
         "--target-dir", str(build_dir), *features,
         "--", "-Z", "unstable-options", "--output-format", "json"],
        cwd=ROOT, check=True, env=full_env,
    )
    dest = WORK / package / config
    dest.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(build_dir / target / "doc" / f"{package.replace('-', '_')}.json",
                    dest / "rustdoc.json")


def listing_paths(text):
    """Public type and module paths in a cargo-public-api listing.

    Associated types also print as `pub type`; a type alias counts only when
    its parent is a listed module.
    """
    paths, aliases = set(), set()
    pattern = re.compile(
        r"^(?:#\[[^\]]*\] )*pub (?:unsafe )?(struct|enum|trait|type|mod|union) "
        r"([A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*)")
    for line in text.splitlines():
        m = pattern.match(line)
        if m:
            (aliases if m.group(1) == "type" else paths).add(m.group(2))
    modules = {line[len("pub mod "):] for line in text.splitlines() if line.startswith("pub mod ")}
    paths |= {a for a in aliases if a.rsplit("::", 1)[0] in modules}
    return paths


# Rendering of rustdoc JSON types (format version 61).

def render_args(args):
    if not args:
        return ""
    if "angle_bracketed" in args:
        ab = args["angle_bracketed"]
        parts = []
        for a in ab["args"]:
            if "type" in a:
                parts.append(render(a["type"]))
            elif "lifetime" in a:
                parts.append(a["lifetime"])
            elif "const" in a:
                parts.append(str(a["const"].get("expr", "_")))
            else:
                parts.append("_")
        for c in ab.get("constraints", []):
            b = c.get("binding", {})
            if "equality" in b:
                eq = b["equality"]
                parts.append(f"{c['name']} = {render(eq['type']) if 'type' in eq else eq}")
            elif "constraint" in b:
                parts.append(f"{c['name']}: {render_bounds(b['constraint'])}")
        return f"<{', '.join(parts)}>" if parts else ""
    if "parenthesized" in args:
        p = args["parenthesized"]
        out = f"({', '.join(render(t) for t in p['inputs'])})"
        if p.get("output") is not None:
            out += f" -> {render(p['output'])}"
        return out
    return ""


PATHS = {}


def render_path(p):
    name = p["path"]
    # A bare `Error` is ambiguous (std::io::Error, fmt::Error, a crate's own);
    # name it by its full path.
    if name == "Error" and str(p.get("id")) in PATHS:
        name = "::".join(PATHS[str(p["id"])]["path"])
    return name + render_args(p.get("args"))


def render_bounds(bounds):
    parts = []
    for b in bounds:
        if "trait_bound" in b:
            tb = b["trait_bound"]
            mod = {"maybe": "?", "maybe_const": "~const "}.get(tb.get("modifier"), "")
            parts.append(mod + render_path(tb["trait"]))
        elif "outlives" in b:
            parts.append(b["outlives"])
        else:
            parts.append("use<..>")
    return " + ".join(parts)


def render(t):
    if t is None:
        return "()"
    (kind, v), = t.items()
    if kind == "resolved_path":
        return render_path(v)
    if kind == "generic":
        return v
    if kind == "primitive":
        return v
    if kind == "tuple":
        return f"({', '.join(render(x) for x in v)})"
    if kind == "slice":
        return f"[{render(v)}]"
    if kind == "array":
        return f"[{render(v['type'])}; {v['len']}]"
    if kind == "borrowed_ref":
        lt = f"{v['lifetime']} " if v.get("lifetime") else ""
        return f"&{lt}{'mut ' if v['is_mutable'] else ''}{render(v['type'])}"
    if kind == "raw_pointer":
        return f"*{'mut' if v['is_mutable'] else 'const'} {render(v['type'])}"
    if kind == "impl_trait":
        return f"impl {render_bounds(v)}"
    if kind == "dyn_trait":
        traits = " + ".join(render_path(p["trait"]) for p in v["traits"])
        if v.get("lifetime"):
            traits += f" + {v['lifetime']}"
        return f"dyn {traits}"
    if kind == "qualified_path":
        if not v.get("trait") or not v["trait"]["path"]:
            return f"{render(v['self_type'])}::{v['name']}"
        return f"<{render(v['self_type'])} as {render_path(v['trait'])}>::{v['name']}"
    if kind == "function_pointer":
        sig = v["sig"]
        out = f"fn({', '.join(render(x[1]) for x in sig['inputs'])})"
        if sig.get("output") is not None:
            out += f" -> {render(sig['output'])}"
        return out
    if kind == "infer":
        return "_"
    if kind == "pat":
        return render(v["type"])
    return f"<{kind}>"


def render_generics(g):
    params = []
    for p in g.get("params", []):
        k = p["kind"]
        if "type" in k:
            if k["type"].get("is_synthetic"):
                continue
            b = render_bounds(k["type"]["bounds"])
            params.append(f"{p['name']}: {b}" if b else p["name"])
        elif "lifetime" in k:
            params.append(p["name"])
        elif "const" in k:
            params.append(f"const {p['name']}: {render(k['const']['type'])}")
    where = []
    for w in g.get("where_predicates", []):
        if "bound_predicate" in w:
            bp = w["bound_predicate"]
            where.append(f"{render(bp['type'])}: {render_bounds(bp['bounds'])}")
        elif "lifetime_predicate" in w:
            lp = w["lifetime_predicate"]
            where.append(f"{lp['lifetime']}: {' + '.join(lp['outlives'])}")
        elif "eq_predicate" in w:
            ep = w["eq_predicate"]
            where.append(f"{render(ep['lhs'])} = {render(ep['rhs'].get('type'))}")
    out = f"<{', '.join(params)}>" if params else ""
    if where:
        out += f" where {', '.join(where)}"
    return out


def type_ids(t, acc):
    """Collects every local or external id a type names."""
    if isinstance(t, dict):
        if "resolved_path" in t and t["resolved_path"].get("id") is not None:
            acc.add(t["resolved_path"]["id"])
        if "trait" in t and isinstance(t["trait"], dict) and t["trait"].get("id") is not None:
            acc.add(t["trait"]["id"])
        for v in t.values():
            type_ids(v, acc)
    elif isinstance(t, list):
        for v in t:
            type_ids(v, acc)
    return acc


def walk_types(t):
    """Yields every nested type node."""
    if isinstance(t, dict):
        if len(t) == 1 and next(iter(t)) in {
            "resolved_path", "generic", "primitive", "tuple", "slice", "array",
            "borrowed_ref", "raw_pointer", "impl_trait", "dyn_trait",
            "qualified_path", "function_pointer", "infer", "pat",
        }:
            yield t
        for v in t.values():
            yield from walk_types(v)
    elif isinstance(t, list):
        for v in t:
            yield from walk_types(v)


def attrs_of(item):
    out = []
    for a in item.get("attrs", []):
        if a == "non_exhaustive":
            out.append("#[non_exhaustive]")
        elif isinstance(a, dict) and "must_use" in a:
            reason = a["must_use"].get("reason")
            out.append(f'#[must_use = "{reason}"]' if reason else "#[must_use]")
    return out


class Crate:
    """One configuration's rustdoc JSON with its public items resolved."""

    def __init__(self, path):
        self.data = json.loads(Path(path).read_text())
        self.index = self.data["index"]
        self.paths = self.data["paths"]
        PATHS.clear()
        PATHS.update(self.paths)
        self.name = self.index[str(self.data["root"])]["name"]
        # id -> list of public paths; external re-exports kept by path.
        self.public = defaultdict(list)
        self.external = []
        self._walk(self.data["root"], [self.name], set())

    def item(self, i):
        return self.index.get(str(i))

    def _walk(self, mod_id, prefix, seen):
        if mod_id in seen:
            return
        seen = seen | {mod_id}
        mod = self.item(mod_id)
        self.public[mod_id].append("::".join(prefix))
        for child in mod["inner"]["module"]["items"]:
            it = self.item(child)
            if it is None or it["visibility"] != "public":
                continue
            kind = next(iter(it["inner"]))
            if kind == "use":
                u = it["inner"]["use"]
                target = self.item(u["id"]) if u.get("id") is not None else None
                if target is None:
                    self.external.append(("::".join(prefix), u["source"], u["is_glob"]))
                    continue
                if u["is_glob"]:
                    if "module" in target["inner"]:
                        for c in target["inner"]["module"]["items"]:
                            ci = self.item(c)
                            if ci and ci["visibility"] == "public" and ci.get("name"):
                                self._add(c, ci, prefix + [ci["name"]], seen)
                    continue
                self._add(u["id"], target, prefix + [u["name"]], seen)
            elif it.get("name"):
                self._add(child, it, prefix + [it["name"]], seen)

    def _add(self, i, it, path, seen):
        if "module" in it["inner"]:
            self._walk(i, path, seen)
        elif "use" in it["inner"]:
            u = it["inner"]["use"]
            target = self.item(u["id"]) if u.get("id") is not None else None
            if target is None:
                self.external.append(("::".join(path[:-1]), u["source"], u["is_glob"]))
            else:
                self._add(u["id"], target, path[:-1] + [u["name"]], seen)
        else:
            self.public[i].append("::".join(path))

    def kind(self, i):
        return next(iter(self.item(i)["inner"]))

    def path_of(self, i):
        ps = self.public.get(i)
        if ps:
            # The shortest path, then the alphabetically first: the root
            # path rather than the prelude's when both exist.
            return sorted(ps, key=lambda p: (p.count("::"), p))[0]
        p = self.paths.get(str(i))
        return "::".join(p["path"]) if p else f"<id {i}>"

    def in_prelude(self, i):
        return any("::prelude::" in p for p in self.public.get(i, []))

    def span(self, i):
        s = self.item(i).get("span")
        return f"{s['filename']}:{s['begin'][0]}" if s else "-"

    def types(self):
        for i, ps in self.public.items():
            if self.kind(i) in {"struct", "enum", "trait", "type_alias", "union"}:
                yield i

    def impls(self, i):
        inner = self.item(i)["inner"]
        k = next(iter(inner))
        return [self.item(x) for x in inner[k].get("impls", []) if self.item(x)]


def fn_sig(c, f_id, owner):
    f = c.item(f_id)
    fn = f["inner"]["function"]
    sig = fn["sig"]
    params = ", ".join(
        (n if n == "self" and render(t) == "Self" else
         ("&self" if n == "self" and render(t) == "&Self" else
          ("&mut self" if n == "self" and render(t) == "&mut Self" else f"{n}: {render(t)}")))
        for n, t in sig["inputs"])
    out = f" -> {render(sig['output'])}" if sig.get("output") is not None else ""
    return f"{owner}::{f['name']}{render_generics(fn['generics'])}({params}){out}"


def receiver(f):
    inputs = f["inner"]["function"]["sig"]["inputs"]
    if not inputs or inputs[0][0] != "self":
        return None
    return render(inputs[0][1])


def returns_self(c, f, owner_id):
    """'Self', 'Result<Self>', 'Option<Self>' or None."""
    out = f["inner"]["function"]["sig"].get("output")
    if out is None:
        return None

    def is_self(t):
        if t == {"generic": "Self"}:
            return True
        rp = t.get("resolved_path")
        return bool(rp and rp.get("id") == owner_id)

    if is_self(out):
        return "Self"
    rp = out.get("resolved_path")
    if rp and rp.get("args") and "angle_bracketed" in rp["args"]:
        args = [a.get("type") for a in rp["args"]["angle_bracketed"]["args"]]
        name = rp["path"].split("::")[-1]
        if args and args[0] is not None and is_self(args[0]) and name in {"Result", "Option"}:
            return f"{name}<Self>"
    return None


def result_error(c, t):
    """The error type of a Result-like return, resolving local aliases."""
    if t is None or "resolved_path" not in t:
        return None
    rp = t["resolved_path"]
    name = rp["path"].split("::")[-1]
    args = []
    if rp.get("args") and "angle_bracketed" in rp["args"]:
        args = [a.get("type") for a in rp["args"]["angle_bracketed"]["args"] if "type" in a]
    if name == "Result" and len(args) == 2:
        return args[1]
    if name == "Result" and len(args) == 1:
        alias = c.item(rp.get("id"))
        if alias and "type_alias" in alias["inner"]:
            inner = alias["inner"]["type_alias"]["type"]
            e = result_error(c, inner)
            return e
    return None


def trait_name(imp):
    t = imp["inner"]["impl"].get("trait")
    return t["path"].split("::")[-1] if t else None


def collect(c, config, rows):
    """Adds one configuration's candidates to rows: list name -> key -> (text, configs)."""

    def add(listname, key, text):
        entry = rows[listname].setdefault(key, [text, set()])
        entry[1].add(config)

    for t_id in c.types():
        it = c.item(t_id)
        tpath = c.path_of(t_id)
        kind = c.kind(t_id)
        attrs = " ".join(attrs_of(it))
        gen = render_generics(it["inner"][kind].get("generics", {})) if kind != "type_alias" else ""
        info = rows["_types"].setdefault(tpath, {
            "head": f"{tpath}{gen}\t{kind}\t{attrs or '-'}\t{c.span(t_id)}\t"
                    f"prelude: {'yes' if c.in_prelude(t_id) else 'no'}",
            "kind": kind, "configs": set(), "traits": {}, "autos": {}, "methods": {},
            "members": {},
        })
        info["configs"].add(config)

        def note(table, key, text=None):
            entry = info[table].setdefault(key, [text or key, set()])
            entry[1].add(config)

        methods, impl_methods = [], []
        for imp in c.impls(t_id) if kind != "trait" and kind != "type_alias" else []:
            ii = imp["inner"]["impl"]
            if ii.get("blanket_impl") is not None:
                continue
            tn = trait_name(imp)
            if tn is None:
                methods += [m for m in ii["items"] if c.item(m) and "function" in c.item(m)["inner"]
                            and c.item(m)["visibility"] == "public"]
                for k in ii["items"]:
                    ac = c.item(k)
                    if ac and "assoc_const" in ac["inner"] and ac["visibility"] == "public":
                        detail = f"{render(ac['inner']['assoc_const']['type'])}"
                        value = ac["inner"]["assoc_const"].get("value")
                        if value:
                            detail += f" = {value}"
                        note("members", f"const {ac['name']}", f"    const {ac['name']}: {detail}")
                        add("constants", (tpath, ac["name"]),
                            f"{tpath}::{ac['name']}: {detail}\t{c.span(k)}")
                continue
            cond = render_generics(ii["generics"])
            trait = ("!" if ii["is_negative"] else "") + render_path(ii["trait"])
            target = ii["for"].get("resolved_path", {}).get("id")
            full = f"impl {trait} for {render(ii['for'])}" + (f" [{cond}]" if cond else "")
            if tn in CONVERSION_TRAITS:
                add("conversion-traits", full, f"{full}\t{c.span(imp['id'])}")
            if target != t_id:
                # rustdoc also lists an impl under a type its trait arguments
                # name, such as `From<ChoiceId> for AnyValue` under ChoiceId.
                note("traits", f"(for another type) {full}")
                # Fallible conversions into foreign types (TryFrom<AnyValue>
                # for f32) have no public type of their own to list them under.
                if tn in {"TryFrom", "FromStr"} and target is None:
                    for m in ii["items"]:
                        f = c.item(m)
                        if not f or "function" not in f["inner"]:
                            continue
                        err = result_error(c, f["inner"]["function"]["sig"].get("output"))
                        if err is not None:
                            sig = fn_sig(c, m, f"<{render(ii['for'])} as {trait}>")
                            docs = f.get("docs") or ""
                            add("errors", (render(err), sig),
                                f"{render(err)}\t# Errors: {'yes' if '# Errors' in docs else 'NO'}"
                                f"\t{sig}\t(trait impl)\t{c.span(m)}")
                continue
            label = trait + (f" [{cond}]" if cond else "")
            note("autos" if ii["is_synthetic"] or tn in AUTO_TRAITS else "traits", label)
            if tn == "Error":
                add("errors", (tpath, "impl Error"), f"{tpath}\timpl Error\t{c.span(imp['id'])}")
            # Component implementations are the widget contract itself; their
            # methods are reviewed once, at the trait's declaration.
            if ("automatically_derived" not in imp.get("attrs", []) and not ii["is_synthetic"]
                    and tn != "Component"):
                impl_methods += [(m, trait) for m in ii["items"]
                                 if c.item(m) and "function" in c.item(m)["inner"]]
        if kind == "trait":
            methods = [m for m in it["inner"]["trait"]["items"]
                       if c.item(m) and "function" in c.item(m)["inner"]]
        if attrs:
            add("attributes", tpath, f"{tpath}\t{kind}\t{attrs}\t{c.span(t_id)}")
        if kind in {"enum", "struct"}:
            closed = "closed" in (it.get("docs") or "").lower() or "exhaustive" in (it.get("docs") or "").lower()
            body = it["inner"][kind]
            if kind == "struct":
                sk = body["kind"]
                fields = []
                if isinstance(sk, dict) and "plain" in sk:
                    fields = sk["plain"]["fields"]
                elif isinstance(sk, dict) and "tuple" in sk:
                    fields = [f for f in sk["tuple"] if f is not None]
                pub = [c.item(f) for f in fields if c.item(f) and c.item(f)["visibility"] == "public"]
                for fl in pub:
                    note("members", f"field {fl['name']}",
                         f"    field {fl['name']}: {render(fl['inner']['struct_field'])}")
                    add("fields", (tpath, fl["name"]),
                        f"{tpath}.{fl['name']}: {render(fl['inner']['struct_field'])}\t{c.span(fl['id'])}")
                if pub:
                    add("non-exhaustive", tpath,
                        f"{tpath}\tstruct with public fields\t{attrs or '-'}\t"
                        f"docs mention closed/exhaustive: {'yes' if closed else 'no'}\t{c.span(t_id)}")
            else:
                for v_id in body["variants"]:
                    v = c.item(v_id)
                    vk = v["inner"]["variant"]["kind"]
                    payload = ""
                    if isinstance(vk, dict) and "tuple" in vk:
                        payload = "(" + ", ".join(
                            render(c.item(f)["inner"]["struct_field"])
                            for f in vk["tuple"] if f is not None and c.item(f)) + ")"
                    elif isinstance(vk, dict) and "struct" in vk:
                        payload = " { " + ", ".join(
                            f"{c.item(f)['name']}: {render(c.item(f)['inner']['struct_field'])}"
                            for f in vk["struct"]["fields"] if c.item(f)) + " }"
                    note("members", f"variant {v['name']}", f"    variant {v['name']}{payload}")
                add("non-exhaustive", tpath,
                    f"{tpath}\tenum ({len(body['variants'])} variants)\t{attrs or '-'}\t"
                    f"docs mention closed/exhaustive: {'yes' if closed else 'no'}\t{c.span(t_id)}")
        for m in methods:
            f = c.item(m)
            sig = fn_sig(c, m, tpath)
            fattrs = " ".join(attrs_of(f))
            loc = c.span(m)
            note("methods", sig, f"    {sig}\t{fattrs or '-'}\t{loc}")
            fn_candidates(c, add, t_id, tpath, f, sig, fattrs, loc, kind)
        # Hand-written trait implementations: their methods (Debug::fmt,
        # TryFrom::try_from, FromStr::from_str and the like) are reviewed with
        # the type, and their Results join the error list.
        for m, trait in impl_methods:
            f = c.item(m)
            sig = fn_sig(c, m, f"<{tpath} as {trait}>")
            loc = c.span(m)
            note("methods", sig, f"    {sig}\t(trait impl)\t{loc}")
            err = result_error(c, f["inner"]["function"]["sig"].get("output"))
            if err is not None:
                docs = f.get("docs") or ""
                add("errors", (render(err), sig),
                    f"{render(err)}\t# Errors: {'yes' if '# Errors' in docs else 'NO'}\t"
                    f"{sig}\t(trait impl)\t{loc}")

    for i in list(c.public):
        kind = c.kind(i)
        if kind in {"function", "constant", "static", "module", "macro"}:
            it = c.item(i)
            if kind == "constant":
                detail = render(it["inner"]["constant"]["type"])
            elif kind == "static":
                detail = render(it["inner"]["static"]["type"])
            elif kind == "function":
                detail = fn_sig(c, i, c.path_of(i).rsplit("::", 1)[0])
            else:
                detail = "-"
            add("free-items", c.path_of(i),
                f"{c.path_of(i)}\t{kind}\t{detail}\t{c.span(i)}\t"
                f"prelude: {'yes' if c.in_prelude(i) else 'no'}")
        if kind == "function":
            f = c.item(i)
            sig = fn_sig(c, i, c.path_of(i).rsplit("::", 1)[0])
            fn_candidates(c, add, None, None, f, sig, " ".join(attrs_of(f)), c.span(i), None)


def fn_candidates(c, add, owner_id, owner, f, sig, fattrs, loc, owner_kind):
    name = f["name"]
    key = sig
    row = f"{sig}\t{fattrs or '-'}\t{loc}"
    recv = receiver(f)
    fn = f["inner"]["function"]
    if fattrs:
        add("attributes", key, f"{sig}\tfunction\t{fattrs}\t{loc}")
    if owner_id is not None and recv is None:
        rs = returns_self(c, f, owner_id)
        if rs:
            m = re.match(r"(try_|from_|with_)", name)
            exact = name in {"new", "bound", "for_parameter", "default"}
            prefix = name if exact else (m.group(1) if m else "other")
            add("constructors", (prefix, key), f"[{prefix}] returns {rs}\t{row}")
        ids = type_ids(fn["sig"]["inputs"], set())
        names = {c.paths.get(str(x), {}).get("path", ["?"])[-1] for x in ids}
        names |= {re.sub(r"<.*", "", render(t)).split("::")[-1]
                  for _, t in fn["sig"]["inputs"]}
        bound_names = set()
        for tnode in walk_types(fn["generics"]):
            if "impl_trait" in tnode:
                bound_names |= {b["trait_bound"]["trait"]["path"].split("::")[-1]
                                for b in tnode["impl_trait"] if "trait_bound" in b}
        for p in fn["generics"].get("params", []):
            if "type" in p["kind"]:
                bound_names |= {b["trait_bound"]["trait"]["path"].split("::")[-1]
                                for b in p["kind"]["type"]["bounds"] if "trait_bound" in b}
        if any(STATE_NAMES.search(n) for n in names | bound_names):
            order = ", ".join(n for n, _ in fn["sig"]["inputs"])
            add("state-owners", key, f"params({order})\t{row}")
    if recv == "Self" and owner_id is not None:
        rs = returns_self(c, f, owner_id)
        if rs == "Self":
            add("builders", key, row)
        elif rs == "Result<Self>":
            add("builders", key, f"[fallible] {row}")
    closures = {b["trait_bound"]["trait"]["path"].split("::")[-1]
                for tnode in walk_types([fn["sig"]["inputs"], fn["generics"]])
                if "impl_trait" in tnode
                for b in tnode["impl_trait"] if "trait_bound" in b}
    for p in fn["generics"].get("params", []):
        if "type" in p["kind"]:
            closures |= {b["trait_bound"]["trait"]["path"].split("::")[-1]
                         for b in p["kind"]["type"]["bounds"] if "trait_bound" in b}
    for w in fn["generics"].get("where_predicates", []):
        if "bound_predicate" in w:
            closures |= {b["trait_bound"]["trait"]["path"].split("::")[-1]
                         for b in w["bound_predicate"]["bounds"] if "trait_bound" in b}
    takes_closure = bool(closures & {"Fn", "FnMut", "FnOnce"}) or any(
        "function_pointer" in t or "dyn_trait" in t and any(
            p["trait"]["path"].split("::")[-1] in {"Fn", "FnMut", "FnOnce"}
            for p in t["dyn_trait"]["traits"])
        for t in walk_types(fn["sig"]["inputs"]))
    if name.startswith("on_") or takes_closure:
        add("callbacks", key, f"[{'on_' if name.startswith('on_') else 'closure'}] {row}")
    inputs = fn["sig"]["inputs"]
    bools = [n for n, t in inputs if t == {"primitive": "bool"}]
    def numeric(t):
        """The numeric primitive of t or of Option<t>, else None."""
        if t.get("primitive") in NUMERIC:
            return t["primitive"]
        rp = t.get("resolved_path")
        if rp and rp["path"].split("::")[-1] == "Option" and rp.get("args"):
            args = rp["args"].get("angle_bracketed", {}).get("args", [])
            if len(args) == 1 and args[0].get("type", {}).get("primitive") in NUMERIC:
                return f"Option<{args[0]['type']['primitive']}>"
        return None

    nums = [f"{n}: {numeric(t)}" for n, t in inputs if numeric(t)]
    out = fn["sig"].get("output") or {}
    ret = numeric(out) if out else None
    builder = recv == "Self" and out == {"generic": "Self"}
    if bools:
        add("bool-params", key, f"bools={len(bools)} builder={'yes' if builder else 'no'}\t{row}")
    if nums or ret:
        add("numeric", key, f"params[{', '.join(nums)}] returns {ret or '-'}\t{row}")
    if name == "get" or name.startswith("get_"):
        add("getters", key, row)
    if re.match(r"(as_|to_|into_)", name) or name in {"iter", "iter_mut", "into_iter"} \
            or name.startswith("iter"):
        add("conversion-methods", key, f"receiver={recv or 'none'}\t{row}")
    err = result_error(c, fn["sig"].get("output"))
    if err is not None:
        docs = f.get("docs") or ""
        add("errors", (render(err), key),
            f"{render(err)}\t# Errors: {'yes' if '# Errors' in docs else 'NO'}\t{row}")


def write_lists(rows, all_configs, reexported):
    cand = OUT / "candidates"
    sem = OUT / "semantic"
    for d in (cand, sem):
        if d.exists():
            shutil.rmtree(d)
        d.mkdir(parents=True)
    headers = {
        "types": "Every public type: path+generics, kind, attributes, definition, "
                 "whether the package's prelude holds it, "
                 "trait impls (conditional ones with [bounds]; impls rustdoc files under "
                 "the type although they are for another type are marked so), "
                 "auto-trait impls (! = negative). Per-configuration differences are "
                 "marked on each trait.",
        "attributes": "Every #[must_use] and #[non_exhaustive] on a public type or function.",
        "constructors": "Associated functions without a receiver returning Self, "
                        "Result<Self, _> or Option<Self>, grouped by name prefix.",
        "state-owners": "Associated functions whose parameter types or bounds end in "
                        "StateOwner, Runtime, Signal, Selection, Controller, Host, Session, "
                        "Binding or State, or are NavigationHistory, ToastQueue, SeriesSource "
                        "or Parameter, with parameter order. Free-function constructors "
                        "are in free-items.txt.",
        "builders": "Methods taking self and returning Self, and [fallible] ones "
                    "returning Result<Self, _>.",
        "callbacks": "Every on_* function, and every function taking a closure "
                     "(Fn, FnMut, FnOnce, a function pointer or a boxed closure), "
                     "with its closure bounds.",
        "bool-params": "Functions with bool parameters; builder = takes self and returns Self "
                       "(a mechanical mark: an operation computing a new value also matches).",
        "numeric": "Functions with bare numeric parameters or return values, "
                   "including Option<number>.",
        "constants": "Every public associated constant, with its type and value.",
        "getters": "Functions named get or get_*.",
        "conversion-methods": "as_*, to_*, into_* and iter* methods with their receivers.",
        "conversion-traits": "From, TryFrom, AsRef, AsMut and FromStr implementations "
                             "(Into/TryInto when written by hand), as impl Trait for Target.",
        "errors": "Error types of public Results (first column) with # Errors coverage, "
                  "including hand-written trait implementations such as TryFrom and "
                  "FromStr, and every std::error::Error implementation.",
        "fields": "Every public field.",
        "free-items": "Every public item that is not a type or a method: free "
                      "functions, constants, statics and modules, with whether the "
                      "prelude holds it.",
        "non-exhaustive": "Every public enum, and every struct with public fields, with its "
                          "attributes and whether its rustdoc mentions closed/exhaustive.",
    }
    def marker(cfgs, configs):
        return "" if cfgs == set(configs) else f"\t[configs: {', '.join(sorted(cfgs))}]"

    for package, lists in rows.items():
        configs = all_configs[package]
        types = lists.pop("_types", {})
        with (cand / "types.txt").open("a" if (cand / "types.txt").exists() else "w") as fh:
            if fh.tell() == 0:
                fh.write(f"# {headers['types']}\n# A trailing [configs: ...] means the "
                         "entry exists only in those configurations.\n")
            fh.write(f"\n## {package}\n")
            for tpath in sorted(types):
                t = types[tpath]
                traits = ", ".join(f"{k}{marker(v[1], t['configs']).replace(chr(9), ' ')}"
                                   for k, v in sorted(t["traits"].items())) or "-"
                autos = ", ".join(f"{k}{marker(v[1], t['configs']).replace(chr(9), ' ')}"
                                  for k, v in sorted(t["autos"].items())) or "-"
                head = t["head"]
                via = reexported.get(tpath.rsplit("::", 1)[-1])
                if via and via[0] != package:
                    head = head.replace("prelude: no", f"prelude: yes (via {via[1]})")
                fh.write(f"{head}\ttraits: {traits}\tauto: {autos}"
                         f"{marker(t['configs'], configs)}\n")
        with (sem / f"{package}.txt").open("w") as fh:
            fh.write(f"# {package}: every public type with its public fields, its "
                     "variants and their payloads,\n# its inherent methods, its trait-declared "
                     "methods and the methods of its hand-written\n# trait implementations. "
                     "Classify each type's role with a source reference,\n# or write "
                     "'unresolved'.\n\n## {package}\n")
            for tpath in sorted(types):
                t = types[tpath]
                kind = t["kind"]
                head = t["head"].split("\t")
                fh.write(f"{tpath}\t{kind}\trole: \t{head[3]}{marker(t['configs'], configs)}\n")
                for key in sorted(t["members"]):
                    text, cfgs = t["members"][key]
                    fh.write(f"{text}{marker(cfgs, t['configs'])}\n")
                for sig in sorted(t["methods"]):
                    text, cfgs = t["methods"][sig]
                    fh.write(f"{text}{marker(cfgs, t['configs'])}\n")
        for name, entries in lists.items():
            f = cand / f"{name}.txt"
            mode = "a" if f.exists() else "w"
            with f.open(mode) as fh:
                if mode == "w":
                    fh.write(f"# {headers[name]}\n# A trailing [configs: ...] means the "
                             "entry exists only in those configurations.\n")
                fh.write(f"\n## {package}\n")
                for key in sorted(entries, key=str):
                    text, cfgs = entries[key]
                    fh.write(f"{text}{marker(cfgs, configs)}\n")


def main():
    global OUT
    ap = argparse.ArgumentParser()
    ap.add_argument("--skip-build", action="store_true")
    ap.add_argument("--out", type=Path, default=OUT,
                    help="output directory (default: inputs/ beside this script)")
    args = ap.parse_args()
    OUT = args.out.resolve()

    target, tool_version, configs = configurations()
    tool = run(["cargo-public-api", "--version"]).stdout.strip()
    if tool != f"cargo-public-api {tool_version}":
        sys.exit(f"inventory.py: needs cargo-public-api {tool_version}, found {tool!r}")
    rustc = run(["rustc", "-vV"]).stdout
    commit = run(["git", "rev-parse", "HEAD"]).stdout.strip()
    clean = run(["git", "status", "--porcelain"]).stdout.strip() == ""

    provenance = {
        "commit": commit,
        "tree_clean": clean,
        "rustc": rustc.strip().splitlines(),
        "cargo_public_api": tool,
        "target": target,
        "configurations": [],
    }
    rows = defaultdict(lambda: defaultdict(dict))
    all_configs = defaultdict(list)
    reexported = {}
    failures = []
    for package, config, features in configs:
        if not args.skip_build:
            build(package, config, features, target)
        copy = WORK / package / config / "rustdoc.json"
        listing = (ROOT / "api" / package / f"{config}.txt").read_text()
        produced = run(["cargo-public-api", "--color=never", "--simplified",
                        "--rustdoc-json", str(copy)]).stdout
        if produced != listing:
            failures.append(f"{package}/{config}: cargo-public-api output differs from api/")
        c = Crate(copy)
        found = {p for i in c.types() for p in c.public[i]}
        found |= {p for i in c.public if c.kind(i) == "module" for p in c.public[i]}
        expected = listing_paths(listing)
        if found != expected:
            failures.append(
                f"{package}/{config}: paths only found here {sorted(found - expected)[:10]}, "
                f"only in the listing {sorted(expected - found)[:10]}")
        provenance["configurations"].append({
            "package": package,
            "configuration": config,
            "features": features,
            "format_version": c.data["format_version"],
            "crate_version": c.data.get("crate_version"),
            "rustdoc_json_sha256": hashlib.sha256(copy.read_bytes()).hexdigest(),
            "listing": f"api/{package}/{config}.txt",
            "external_reexports": sorted({f"{m} <- {s}{'::*' if g else ''}"
                                          for m, s, g in c.external}),
        })
        all_configs[package].append(config)
        collect(c, config, rows[package])
        # Items another package's prelude re-exports by name, such as the
        # core types in pert-ui-runtime's prelude.
        # The source path may be `crate::Name`, so the item is matched by name
        # in the other packages.
        for module, source, glob in c.external:
            if module.endswith("::prelude") and not glob:
                reexported[source.rsplit("::", 1)[-1]] = (package, module)
        print(f"{package}/{config}: {len(found)} paths", file=sys.stderr)
    if failures:
        sys.exit("inventory.py: the inputs do not match the listings:\n" + "\n".join(failures))
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    write_lists(rows, all_configs, reexported)


if __name__ == "__main__":
    main()
