#!/usr/bin/env python3
"""Parse choreo JSON test reports and print a summary."""

import argparse
import json
import glob
import os
import sys

def find_report(path: str) -> str:
    if os.path.isfile(path):
        return path
    if os.path.isdir(path):
        files = sorted(
            glob.glob(os.path.join(path, "choreo_test_report_*.json")),
            key=os.path.getmtime,
            reverse=True,
        )
        if files:
            return files[0]
        print(f"No report files found in directory: {path}", file=sys.stderr)
        sys.exit(1)
    print(f"Path not found: {path}", file=sys.stderr)
    sys.exit(1)


def parse_report(data: list) -> dict:
    tests = sum(f.get("summary", {}).get("tests", 0) for f in data)
    failures = sum(f.get("summary", {}).get("failures", 0) for f in data)
    time_s = sum(f.get("summary", {}).get("totalTimeInSeconds", 0) for f in data)

    failing_steps = []
    for feature in data:
        uri = feature.get("uri", "")
        for scenario in feature.get("elements", []):
            steps = scenario.get("steps", []) + scenario.get("after", [])
            for step in steps:
                status = step.get("result", {}).get("status", "")
                if status != "passed":
                    failing_steps.append(
                        {
                            "uri": uri,
                            "scenario": scenario.get("name", ""),
                            "step": step.get("name", ""),
                            "status": status or "unknown",
                        }
                    )

    return {
        "totals": {"tests": tests, "failures": failures, "time_s": time_s},
        "failing_steps": failing_steps,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "report_path",
        nargs="?",
        default="reports",
        help="JSON report file or directory (default: ./reports)",
    )
    parser.add_argument(
        "--json", action="store_true", dest="json_output", help="Output as JSON"
    )
    args = parser.parse_args()

    report_file = find_report(args.report_path)
    print(f"Report: {report_file}", file=sys.stderr)

    with open(report_file) as f:
        data = json.load(f)

    result = parse_report(data)

    if args.json_output:
        result["report"] = report_file
        json.dump(result, sys.stdout, indent=2)
        print()
    else:
        t = result["totals"]
        print(f"tests={t['tests']} failures={t['failures']} time_s={t['time_s']}")
        if result["failing_steps"]:
            print("Failing steps:")
            for s in result["failing_steps"]:
                print(f"  {s['uri']} :: {s['scenario']} :: {s['step']} [{s['status']}]")
        else:
            print("No failing steps.")


if __name__ == "__main__":
    main()

