"""Evaluation reporting and trend dashboard generator (CSV, JSON, Markdown)."""

import csv
import json
import os
import subprocess
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Optional


@dataclass
class MetricRecord:
    """Individual metric evaluation record."""

    category: str
    fixture_id: str
    metric: str
    value: float
    target: Optional[float] = None
    passed: Optional[bool] = None
    unit: str = ""
    notes: str = ""
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())


class EvaluationReport:
    """Container for collecting and publishing evaluation benchmark reports."""

    def __init__(self, run_id: Optional[str] = None):
        self.run_id = run_id or datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
        self.commit = self._get_git_commit()
        self.records: list[MetricRecord] = []

    def _get_git_commit(self) -> str:
        try:
            res = subprocess.run(
                ["git", "rev-parse", "--short", "HEAD"],
                capture_output=True,
                text=True,
                check=False,
            )
            return res.stdout.strip() or "unknown"
        except Exception:
            return "unknown"

    def add_result(
        self,
        category: str,
        fixture_id: str,
        metric: str,
        value: float,
        target: Optional[float] = None,
        unit: str = "",
        higher_is_better: bool = True,
        notes: str = "",
    ):
        """Record an evaluation metric result."""
        passed = None
        if target is not None:
            if higher_is_better:
                passed = value >= target
            else:
                passed = value <= target

        record = MetricRecord(
            category=category,
            fixture_id=fixture_id,
            metric=metric,
            value=round(float(value), 4),
            target=round(float(target), 4) if target is not None else None,
            passed=passed,
            unit=unit,
            notes=notes,
        )
        self.records.append(record)

    def to_dict(self) -> dict[str, Any]:
        """Convert entire report to serializable dictionary."""
        return {
            "run_id": self.run_id,
            "commit": self.commit,
            "generated_at": datetime.now(timezone.utc).isoformat(),
            "total_metrics": len(self.records),
            "passed_count": sum(1 for r in self.records if r.passed is True),
            "failed_count": sum(1 for r in self.records if r.passed is False),
            "records": [asdict(r) for r in self.records],
        }

    def to_json(self, path: Path | str):
        """Export report as JSON."""
        target_path = Path(path)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        with open(target_path, "w", encoding="utf-8") as f:
            json.dump(self.to_dict(), f, indent=2)

    def to_csv(self, path: Path | str):
        """Export report records as CSV."""
        target_path = Path(path)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        with open(target_path, "w", newline="", encoding="utf-8") as f:
            writer = csv.writer(f)
            writer.writerow(
                [
                    "run_id",
                    "commit",
                    "category",
                    "fixture_id",
                    "metric",
                    "value",
                    "unit",
                    "target",
                    "passed",
                    "notes",
                    "timestamp",
                ]
            )
            for r in self.records:
                writer.writerow(
                    [
                        self.run_id,
                        self.commit,
                        r.category,
                        r.fixture_id,
                        r.metric,
                        r.value,
                        r.unit,
                        r.target if r.target is not None else "",
                        r.passed if r.passed is not None else "",
                        r.notes,
                        r.timestamp,
                    ]
                )

    def to_markdown_dashboard(self) -> str:
        """Render a formatted Markdown dashboard table of evaluation metrics."""
        lines = [
            f"# Flux Objective Quality Dashboard — Run `{self.run_id}`",
            "",
            f"**Commit:** `{self.commit}`  ",
            f"**Date:** {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')}  ",
            f"**Total Metrics Evaluated:** {len(self.records)}  ",
            "",
            "| Category | Fixture | Metric | Measured | Target | Status | Notes |",
            "|----------|---------|--------|----------|--------|--------|-------|",
        ]

        for r in self.records:
            val_str = f"{r.value} {r.unit}".strip()
            target_str = f"{r.target} {r.unit}".strip() if r.target is not None else "—"
            if r.passed is True:
                status = "PASS"
            elif r.passed is False:
                status = "FAIL"
            else:
                status = "INFO"

            lines.append(
                f"| {r.category} | `{r.fixture_id}` | {r.metric} | {val_str} | {target_str} | {status} | {r.notes} |"
            )

        lines.append("")
        return "\n".join(lines)

    def save_dashboard(self, path: Path | str):
        """Write the markdown dashboard to a file."""
        target_path = Path(path)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        with open(target_path, "w", encoding="utf-8") as f:
            f.write(self.to_markdown_dashboard())
