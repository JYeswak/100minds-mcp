#!/usr/bin/env python3
"""
Consolidate 100minds to exactly 100 thinkers.

First principles approach:
1. Export best 100 unique thinkers from DB
2. Map to 6 clean domains
3. Write canonical JSON files
4. These files become THE source of truth
"""

import json
import os
import sqlite3
from pathlib import Path
from collections import defaultdict

# Domain mapping: messy → clean
DOMAIN_MAP = {
    # Software (20)
    "software": "software",
    "software-architecture": "software",
    "software-development": "software",
    "software-engineering": "software",
    "cloud-computing": "software",
    "computer-architecture": "software",

    # Systems (15)
    "systems": "systems",
    "systems-thinking": "systems",
    "management-theory": "systems",  # Deming, Goldratt, etc. are systems thinkers

    # Business (20)
    "business": "business",
    "business-innovation": "business",
    "productivity": "business",
    "team-organization": "business",

    # Decision-Making (15) - includes AI/ML folks focused on reasoning
    "decision-making": "decision-making",
    "ai-ml": "decision-making",  # They're about reasoning/intelligence

    # Philosophy (15)
    "philosophy": "philosophy",
    "philosophy-ethics": "philosophy",

    # Security (15)
    "security": "security",

    # Special cases
    "entrepreneurship": None,  # Will reassign individually
    "general": None,  # Skip Collective Wisdom
}

# Manual reassignments for "entrepreneurship" grab-bag
ENTREPRENEUR_REMAP = {
    # Software engineers → software
    "donald-knuth": "software",
    "edsger-dijkstra": "software",
    "bjarne-stroustrup": "software",
    "linus-torvalds": "software",
    "rich-hickey": "software",
    "john-ousterhout": "software",
    "tony-hoare": "software",
    "ward-cunningham": "software",
    "michael-feathers": "software",
    "sandi-metz": "software",
    "addy-osmani": "software",
    "dan-abramov": "software",
    "brendan-gregg": "software",
    "martin-kleppmann": "software",
    "michael-nygard": "software",
    "kelsey-hightower": "software",
    "jeff-dean": "software",
    "eric-brewer": "software",
    "grady-booch": "software",
    "andy-hunt": "software",
    "dave-thomas": "software",
    "gene-kim": "software",
    "jez-humble": "software",
    "nicole-forsgren": "software",
    "camille-fournier": "software",

    # Business/Entrepreneurship → business
    "tim-ferriss": "business",
    "guy-kawasaki": "business",
    "seth-godin": "business",
    "darren-hardy": "business",
    "michael-gerber": "business",
    "charlie-munger": "decision-making",  # Investor, mental models
    "nassim-taleb": "decision-making",  # Risk/uncertainty
    "simon-wardley": "business",
    "werner-vogels": "software",  # AWS CTO
}

# Duplicates to skip (keep the one with more principles)
SKIP_IDS = {
    "amodei",  # Keep dario-amodei
    "tegmark-max",  # Keep max-tegmark
    "stuart-russell-ethics",  # Keep stuart-russell
    "edwards-deming",  # Keep w-edwards-deming
}

# Target distribution
TARGET = {
    "software": 20,
    "systems": 15,
    "business": 20,
    "decision-making": 15,
    "philosophy": 15,
    "security": 15,
}

def get_clean_domain(thinker_id: str, messy_domain: str) -> str | None:
    """Map messy domain to clean domain."""
    if thinker_id in SKIP_IDS:
        return None
    if thinker_id in ENTREPRENEUR_REMAP:
        return ENTREPRENEUR_REMAP[thinker_id]
    clean = DOMAIN_MAP.get(messy_domain)
    if clean is None and messy_domain == "entrepreneurship":
        # Default unmapped entrepreneurs to business
        return "business"
    return clean

def main():
    db_path = os.path.expanduser("~/Library/Application Support/100minds/wisdom.db")
    output_dir = Path("/Users/josh/Desktop/Projects/100minds-mcp/data/thinkers")

    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row

    # Get all thinkers with principle counts
    thinkers = conn.execute("""
        SELECT t.id, t.name, t.domain, t.background, t.profile_json,
               COUNT(p.id) as principle_count
        FROM thinkers t
        LEFT JOIN principles p ON t.id = p.thinker_id
        GROUP BY t.id
        ORDER BY principle_count DESC, t.name
    """).fetchall()

    # Group by clean domain
    by_domain = defaultdict(list)
    skipped = []

    for t in thinkers:
        clean_domain = get_clean_domain(t["id"], t["domain"])
        if clean_domain is None:
            skipped.append((t["id"], t["name"], t["domain"]))
            continue
        by_domain[clean_domain].append(dict(t))

    print("=== DOMAIN DISTRIBUTION ===")
    for domain, target in TARGET.items():
        available = len(by_domain[domain])
        print(f"  {domain}: {available} available, target {target}")

    print(f"\n=== SKIPPED ({len(skipped)}) ===")
    for id_, name, domain in skipped[:10]:
        print(f"  {id_}: {name} ({domain})")

    # Select top N from each domain
    selected = []
    for domain, target in TARGET.items():
        candidates = by_domain[domain][:target]
        selected.extend([(domain, c) for c in candidates])
        if len(candidates) < target:
            print(f"WARNING: {domain} has only {len(candidates)}, need {target}")

    print(f"\n=== SELECTED: {len(selected)} thinkers ===")

    # Clear existing JSON files
    for domain_dir in output_dir.iterdir():
        if domain_dir.is_dir():
            for f in domain_dir.glob("*.json"):
                f.unlink()

    # Create domain directories
    for domain in TARGET.keys():
        (output_dir / domain).mkdir(exist_ok=True)

    # Export each thinker
    exported = 0
    for domain, thinker in selected:
        thinker_id = thinker["id"]

        # Get principles
        principles = conn.execute("""
            SELECT name, description, domain_tags
            FROM principles
            WHERE thinker_id = ?
        """, (thinker_id,)).fetchall()

        if not principles:
            print(f"  SKIP {thinker_id}: no principles")
            continue

        # Build canonical JSON
        canonical = {
            "id": thinker_id,
            "name": thinker["name"],
            "domain": domain,
            "background": thinker["background"] or f"Expert in {domain}",
            "principles": []
        }

        for p in principles[:4]:  # Max 4 principles per thinker
            # Parse domain_tags (stored as JSON string)
            try:
                tags = json.loads(p["domain_tags"]) if p["domain_tags"] else [domain]
            except:
                tags = [domain]

            # Extract falsification if embedded in description
            desc = p["description"] or ""
            falsification = None
            if "Falsifiable when:" in desc:
                parts = desc.split("Falsifiable when:")
                desc = parts[0].strip()
                falsification = parts[1].strip() if len(parts) > 1 else None

            canonical["principles"].append({
                "name": p["name"],
                "description": desc,
                "domain_tags": tags[:5],  # Max 5 tags
                "falsification": falsification or f"When {p['name'].lower()} leads to worse outcomes than alternatives"
            })

        # Write file
        out_path = output_dir / domain / f"{thinker_id}.json"
        with open(out_path, "w") as f:
            json.dump(canonical, f, indent=2)

        exported += 1

    print(f"\n=== EXPORTED: {exported} thinkers ===")

    # Final count by domain
    print("\n=== FINAL COUNTS ===")
    total = 0
    for domain in TARGET.keys():
        count = len(list((output_dir / domain).glob("*.json")))
        total += count
        print(f"  {domain}: {count}")
    print(f"  TOTAL: {total}")

    conn.close()

if __name__ == "__main__":
    main()
