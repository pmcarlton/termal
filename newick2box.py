#!/usr/bin/env python3
"""
newick2box.py

Render a Newick tree in the terminal using Unicode box-drawing characters.
Topology only (branch lengths ignored). Tree on the left, leaf labels aligned
to the first column after the tree.

Dependencies:
  - biopython (recommended): pip install biopython
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

try:
    from Bio import Phylo
except ImportError as e:
    raise SystemExit(
        "Biopython is required. Install with: pip install biopython"
    ) from e


# ----------------------------
# Layout data structures
# ----------------------------

@dataclass(frozen=True)
class NodeInfo:
    depth: int
    y: int  # row index


# ----------------------------
# Tree utilities
# ----------------------------

def _is_leaf(clade) -> bool:
    return not getattr(clade, "clades", None)


def _label_for_leaf(clade) -> str:
    # Biopython stores names in clade.name (may be None)
    return str(clade.name) if clade.name is not None else ""


def _collapse_unary(clade):
    """
    Collapse chains of unary internal nodes so depth is not inflated
    by nodes with exactly one child. This reduces horizontal width.
    """
    while getattr(clade, "clades", None) and len(clade.clades) == 1:
        child = clade.clades[0]
        # Merge: carry name if parent has one and child doesn't
        if getattr(clade, "name", None) and not getattr(child, "name", None):
            child.name = clade.name
        clade = child
    if getattr(clade, "clades", None):
        newkids = []
        for k in clade.clades:
            newkids.append(_collapse_unary(k))
        clade.clades = newkids
    return clade


def _assign_rows_and_depths(root) -> Tuple[Dict[int, NodeInfo], List[Tuple[int, str]]]:
    """
    Assign each clade a (depth, y) where leaves occupy consecutive rows.
    Internal nodes get y as the midpoint of their children's rows.

    Returns:
      node_map: dict mapping id(clade) -> NodeInfo
      leaves: list of (leaf_y, leaf_label) in row order
    """
    node_map: Dict[int, NodeInfo] = {}
    leaves: List[Tuple[int, str]] = []
    next_y = 0

    def walk(clade, depth: int) -> int:
        nonlocal next_y
        if _is_leaf(clade):
            y = next_y
            next_y += 1
            node_map[id(clade)] = NodeInfo(depth=depth, y=y)
            leaves.append((y, _label_for_leaf(clade)))
            return y

        child_ys = [walk(ch, depth + 1) for ch in clade.clades]
        y = (min(child_ys) + max(child_ys)) // 2
        node_map[id(clade)] = NodeInfo(depth=depth, y=y)
        return y

    walk(root, 0)
    return node_map, leaves


# ----------------------------
# Rendering
# ----------------------------
def render_box_tree(root, node_map: Dict[int, NodeInfo], leaves: List[Tuple[int, str]]) -> str:
    """
    Render with 2 columns per depth:
      node_x = depth * 2
      connector column = node_x + 1 (junctions + vertical)

    Improvements vs prior version:
      - Uses ┌ for the top child junction row, └ for the bottom, ├ for middle.
      - Extends each leaf's horizontal line all the way to the leaf label.
    """
    if not leaves:
        return ""

    n_rows = max(y for y, _ in leaves) + 1
    max_depth = max(info.depth for info in node_map.values())
    tree_width = max_depth * 2  # last depth node_x
    label_start = tree_width + 2  # first column where label text begins
    label_col = label_start  # grid width up to (but not including) labels; labels appended later

    grid: List[List[str]] = [[" " for _ in range(label_col)] for _ in range(n_rows)]

    def put(y: int, x: int, ch: str):
        if 0 <= y < n_rows and 0 <= x < len(grid[y]):
            existing = grid[y][x]
            if existing == " ":
                grid[y][x] = ch
                return

            # Preserve junction/corner glyphs over plain strokes.
            if existing in {"┌", "└", "├", "┼"}:
                return
            if ch in {"┌", "└", "├", "┼"}:
                grid[y][x] = ch
                return

            # Merge crossings of vertical/horizontal.
            merges = {
                frozenset({"│", "─"}): "┼",
                frozenset({"│", "┼"}): "┼",
                frozenset({"─", "┼"}): "┼",
            }
            merged = merges.get(frozenset({existing, ch}))
            if merged:
                grid[y][x] = merged
            # else: keep existing

    def draw_internal(clade):
        info = node_map[id(clade)]
        if _is_leaf(clade):
            return

        x_node = info.depth * 2
        x_conn = x_node + 1

        kids = clade.clades
        kid_infos = [node_map[id(k)] for k in kids]
        ys = [ki.y for ki in kid_infos]
        y_top, y_bottom = min(ys), max(ys)

        # Draw vertical spine only between top and bottom (exclusive),
        # so endpoints can be proper corners.
        for y in range(y_top + 1, y_bottom):
            put(y, x_conn, "│")

        # Connect each child
        for kid, ki in zip(kids, kid_infos):
            y = ki.y

            if y == y_top and y != y_bottom:
                jch = "┌"
            elif y == y_bottom and y != y_top:
                jch = "└"
            else:
                # If all children are on same row (degenerate), fall back to ├
                # otherwise middle children get ├.
                jch = "├" if y_top != y_bottom else "├"

            put(y, x_conn, jch)

            # Horizontal run from just right of the junction to the child's node column.
            x_child = ki.depth * 2
            for x in range(x_conn + 1, x_child + 1):
                put(y, x, "─")

            draw_internal(kid)

    draw_internal(root)

    # Extend horizontal lines for leaves all the way to the label column (right up to label_start-1).
    # We do this BEFORE writing labels.
    leaf_rows = {y for y, _ in leaves}
    for y in leaf_rows:
        row = grid[y]
        # Find last non-space char in the tree region (0 .. label_start-1).
        last = -1
        for x in range(len(row) - 1, -1, -1):
            if row[x] != " ":
                last = x
                break
        # Fill from last+1 up to label_start-1 with ─ (if there is any gap).
        for x in range(last + 1, label_start):
            put(y, x, "─")

    # Attach leaf labels, aligned
    leaf_by_y = {y: name for y, name in leaves}
    for y in range(n_rows):
        name = leaf_by_y.get(y, "")
        # Ensure grid row is long enough to hold labels
        if name:
            if len(grid[y]) < label_start:
                grid[y].extend([" "] * (label_start - len(grid[y])))
            for i, ch in enumerate(name):
                x = label_start + i
                if x >= len(grid[y]):
                    grid[y].append(ch)
                else:
                    grid[y][x] = ch

    return "\n".join("".join(row).rstrip() for row in grid)

# def render_box_tree(root, node_map: Dict[int, NodeInfo], leaves: List[Tuple[int, str]]) -> str:
    # """
    # Render with 2 columns per depth:
      # node_x = depth * 2
      # horizontal connector occupies node_x+1

    # Drawing strategy:
      # - For each internal node:
          # - draw a vertical '│' in its connector column (node_x+1)
            # spanning from first_child_y .. last_child_y
          # - for each child:
              # - draw horizontal '─' from parent connector column to child node_x
              # - place junction glyph at the parent connector column on child's row:
                    # '├' for intermediate children, '└' for last child
              # - place '┬' at the junction row for the first child if >=2 children,
                # but we encode that via vertical+tee corners (├/└) which is usually sufficient.
      # - Leaves get their label printed starting at label_col = tree_width + 1
    # """
    # if not leaves:
        # return ""

    # n_rows = max(y for y, _ in leaves) + 1
    # max_depth = max(info.depth for info in node_map.values())
    # tree_width = max_depth * 2  # last depth node_x
    # label_col = tree_width + 2  # one space after tree

    # # Build a mutable grid of characters for the tree area.
    # # We'll keep it only up to label_col; labels appended later.
    # grid: List[List[str]] = [[" " for _ in range(label_col)] for _ in range(n_rows)]

    # def put(y: int, x: int, ch: str):
        # if 0 <= y < n_rows and 0 <= x < label_col:
            # existing = grid[y][x]
            # if existing == " ":
                # grid[y][x] = ch
                # return
            # # Merge crossings:
            # # If we already have a vertical and add horizontal (or vice versa), use '┼'
            # # If we already have a junction char, prefer the junction.
            # merges = {
                # frozenset({"│", "─"}): "┼",
                # frozenset({"│", "┼"}): "┼",
                # frozenset({"─", "┼"}): "┼",
            # }
            # if existing in {"├", "└", "┼"}:
                # # keep junction/cross
                # return
            # if ch in {"├", "└", "┼"}:
                # grid[y][x] = ch
                # return
            # merged = merges.get(frozenset({existing, ch}))
            # if merged:
                # grid[y][x] = merged
            # else:
                # # fallback: keep existing
                # pass

    # def draw_internal(clade):
        # info = node_map[id(clade)]
        # if _is_leaf(clade):
            # # node point not strictly necessary for leaves; we rely on connectors
            # return

        # x_node = info.depth * 2
        # x_conn = x_node + 1

        # # Children rows in order
        # kids = clade.clades
        # kid_infos = [node_map[id(k)] for k in kids]
        # ys = [ki.y for ki in kid_infos]
        # y0, y1 = min(ys), max(ys)

        # # vertical spine
        # for y in range(y0, y1 + 1):
            # put(y, x_conn, "│")

        # # connect each child
        # for i, (kid, ki) in enumerate(zip(kids, kid_infos)):
            # y = ki.y
            # # junction at parent connector column
            # jch = "└" if i == len(kids) - 1 else "├"
            # put(y, x_conn, jch)

            # # horizontal run to child node_x
            # x_child = ki.depth * 2
            # for x in range(x_conn + 1, x_child + 1):
                # put(y, x, "─")

            # draw_internal(kid)

    # draw_internal(root)

    # # Attach leaf labels, aligned
    # # Sort by y to ensure row order
    # leaf_by_y = {y: name for y, name in leaves}
    # for y in range(n_rows):
        # name = leaf_by_y.get(y, "")
        # # Ensure there's at least one space between tree and label
        # start = tree_width + 2
        # for i, ch in enumerate(name):
            # if start + i >= len(grid[y]):
                # grid[y].append(ch)
            # else:
                # grid[y][start + i] = ch

    # return "\n".join("".join(row).rstrip() for row in grid)


# ----------------------------
# CLI
# ----------------------------

def main(argv: Optional[List[str]] = None) -> int:
    p = argparse.ArgumentParser(
        description="Render a Newick tree using Unicode box-drawing characters (topology only)."
    )
    p.add_argument("newick", help="Path to Newick file")
    p.add_argument("--no-collapse-unary", action="store_true",
                   help="Do not collapse unary internal nodes (may increase width).")
    p.add_argument("--root", default=None,
                   help="Optional: root the tree at a named clade (exact match). "
                        "If omitted, uses the file's root as-is.")
    args = p.parse_args(argv)

    tree = Phylo.read(args.newick, "newick")
    root = tree.root

    if not args.no_collapse_unary:
        root = _collapse_unary(root)

    if args.root is not None:
        # Find the first clade with matching name and reroot there
        target = None
        for clade in root.find_clades():
            if getattr(clade, "name", None) == args.root:
                target = clade
                break
        if target is None:
            raise SystemExit(f"Could not find clade named '{args.root}' to root on.")
        tree.root_with_outgroup(target)
        root = tree.root
        if not args.no_collapse_unary:
            root = _collapse_unary(root)

    node_map, leaves = _assign_rows_and_depths(root)
    out = render_box_tree(root, node_map, leaves)
    sys.stdout.write(out + ("\n" if out and not out.endswith("\n") else ""))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

