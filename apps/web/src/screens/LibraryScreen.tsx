import React, { useState } from "react";
import { PageLayout, Icon, SegmentedControl, Pill } from "@flux/ui";

interface ProjectRow {
  name: string;
  type: string;
  length: string;
  size: string;
  changed: string;
  storage: "Folder" | "Browser";
}

const PROJECTS: ProjectRow[] = [
  {
    name: "Harbor Lights",
    type: "Music",
    length: "3:42",
    size: "412 MB",
    changed: "12 s ago",
    storage: "Folder",
  },
  {
    name: "Interview ep. 12",
    type: "Podcast",
    length: "48:10",
    size: "1.2 GB",
    changed: "Yesterday",
    storage: "Folder",
  },
  {
    name: "Sunset 8D",
    type: "Spatial",
    length: "4:18",
    size: "380 MB",
    changed: "Mon",
    storage: "Browser",
  },
  {
    name: "Rain voiceover",
    type: "Voice",
    length: "1:06",
    size: "42 MB",
    changed: "2 weeks ago",
    storage: "Browser",
  },
  {
    name: "Lecture 4 raw",
    type: "Lecture",
    length: "1:52:14",
    size: "2.1 GB",
    changed: "Sep 18",
    storage: "Folder",
  },
];

export const LibraryScreen: React.FC = () => {
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState("All");
  const [activeChip, setActiveChip] = useState("My Audio");

  const filtered = PROJECTS.filter((p) => {
    const matchesSearch = p.name.toLowerCase().includes(search.toLowerCase());
    const matchesCategory = category === "All" || p.type.toLowerCase() === category.toLowerCase();
    return matchesSearch && matchesCategory;
  });

  return (
    <PageLayout
      currentTab="library"
      title="Projects"
      actions={
        <button className="btn" type="button">
          <Icon name="folder" />
          <span>Open .flux file</span>
        </button>
      }
    >
      <div className="row">
        <div style={{ flex: 1, minWidth: "200px" }}>
          <input
            className="inp"
            value={search}
            placeholder="Search projects, transcripts and markers"
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <SegmentedControl
          items={["All", "Music", "Podcast", "Spatial"]}
          selectedIndex={0}
          onChange={(item) => setCategory(item)}
        />
      </div>

      <div className="row">
        {["My Audio", "Client work", "Archive", "+ Folder"].map((chip) => (
          <button
            key={chip}
            type="button"
            className={`chip ${chip === activeChip ? "on" : ""}`}
            onClick={() => setActiveChip(chip)}
          >
            {chip}
          </button>
        ))}
      </div>

      <div className="tw">
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Length</th>
              <th>Size</th>
              <th>Changed</th>
              <th>Stored</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {filtered.map((r) => (
              <tr key={r.name}>
                <td>
                  <a className="b7" href="#edit" data-go="edit" style={{ textDecoration: "none" }}>
                    {r.name}
                  </a>
                </td>
                <td className="dim">{r.type}</td>
                <td className="mono">{r.length}</td>
                <td className="mono">{r.size}</td>
                <td className="dim">{r.changed}</td>
                <td>
                  <Pill label={r.storage} variant={r.storage === "Folder" ? "ok" : "warn"} />
                </td>
                <td>
                  <button className="btn g ic" type="button" aria-label="More actions">
                    <Icon name="dots" />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="callout warn">
        <Icon name="warn" />
        <div>
          <b>2 projects are only in this browser.</b> Browsers may clear storage when space is low.{" "}
          <a className="act" href="#export-project" data-go="export-project">
            Back them up as .flux files
          </a>{" "}
          or move them to a folder.
        </div>
      </div>
    </PageLayout>
  );
};
