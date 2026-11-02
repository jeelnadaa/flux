"""Tests for EvaluationReport and dashboard formatting."""

import json
from pathlib import Path
from tools.eval.reporter import EvaluationReport


def test_report_generation(tmp_path: Path):
    report = EvaluationReport(run_id="test_run_01")
    report.add_result(
        category="DSP",
        fixture_id="sine_1k",
        metric="THD+N",
        value=-115.5,
        target=-110.0,
        unit="dBFS",
        higher_is_better=False,
    )
    report.add_result(
        category="MIR",
        fixture_id="test_song",
        metric="MIREX Key",
        value=1.0,
        target=0.7,
        unit="score",
        higher_is_better=True,
    )

    data = report.to_dict()
    assert data["total_metrics"] == 2
    assert data["passed_count"] == 2
    assert data["failed_count"] == 0

    json_path = tmp_path / "report.json"
    report.to_json(json_path)
    assert json_path.exists()

    with open(json_path, encoding="utf-8") as f:
        loaded = json.load(f)
    assert loaded["run_id"] == "test_run_01"

    csv_path = tmp_path / "report.csv"
    report.to_csv(csv_path)
    assert csv_path.exists()

    dash_str = report.to_markdown_dashboard()
    assert "Flux Objective Quality Dashboard" in dash_str
    assert "THD+N" in dash_str
    assert "PASS" in dash_str
