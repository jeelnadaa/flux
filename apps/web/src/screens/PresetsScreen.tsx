import React, { useState, useRef } from "react";
import { PageLayout, SegmentedControl, Input, Tile, Button } from "@flux/ui";

interface PresetItem {
  name: string;
  subtitle: string;
  category: "Effects" | "Chains" | "Stage paths" | "Export";
}

const CATEGORIES = ["Effects", "Chains", "Stage paths", "Export"] as const;

const PRESETS: PresetItem[] = [
  { name: "Clear voice", subtitle: "EQ", category: "Effects" },
  { name: "Late night radio", subtitle: "Chain, 4 effects", category: "Chains" },
  { name: "Tape warmth", subtitle: "Cassette", category: "Effects" },
  { name: "Podcast ready", subtitle: "Chain, 6 effects", category: "Chains" },
  { name: "Slow orbit", subtitle: "Stage path", category: "Stage paths" },
  { name: "Tight drums", subtitle: "Compressor", category: "Effects" },
  { name: "Dream hall", subtitle: "Reverb", category: "Effects" },
  { name: "Spoken word", subtitle: "Chain, 5 effects", category: "Chains" },
];

export const PresetsScreen: React.FC = () => {
  const [presetsList, setPresetsList] = useState<PresetItem[]>(PRESETS);
  const [selectedCategoryIdx, setSelectedCategoryIdx] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [importedNotice, setImportedNotice] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const selectedCategory = CATEGORIES[selectedCategoryIdx];

  const filteredPresets = presetsList.filter((p) => {
    const matchesCategory = selectedCategoryIdx === 0 ? true : p.category === selectedCategory;
    const matchesSearch =
      searchQuery.trim().length === 0 ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.subtitle.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const handleImport = () => {
    fileInputRef.current?.click();
  };

  const handleFileChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
    if (e.target.files && e.target.files[0]) {
      try {
        const text = await e.target.files[0].text();
        const parsed = JSON.parse(text);
        const name = parsed.name || e.target.files[0].name.replace(/\.[^/.]+$/, "");
        const newItem: PresetItem = {
          name,
          subtitle: parsed.category || "Custom Preset",
          category: "Effects",
        };
        setPresetsList((prev) => [newItem, ...prev]);
        setImportedNotice(`Imported "${name}" successfully`);
        setTimeout(() => setImportedNotice(null), 3000);
      } catch {
        setImportedNotice("Failed to parse preset file");
        setTimeout(() => setImportedNotice(null), 3000);
      }
    }
  };

  const handleExport = () => {
    try {
      const data = {
        schemaVersion: 1,
        exportedAt: new Date().toISOString(),
        presets: presetsList,
      };
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "flux-factory-presets.fluxpreset";
      a.click();
      URL.revokeObjectURL(url);
      setImportedNotice("Presets exported successfully");
      setTimeout(() => setImportedNotice(null), 3000);
    } catch {
      setImportedNotice("Export failed");
      setTimeout(() => setImportedNotice(null), 3000);
    }
  };

  return (
    <PageLayout
      title="Presets"
      subtitle="Saved as files you own. Import and export them any time."
      currentTab="presets"
      actions={
        <>
          <input
            ref={fileInputRef}
            type="file"
            accept=".fluxpreset,application/json"
            style={{ display: "none" }}
            onChange={handleFileChange}
          />
          <Button label="Import .fluxpreset" icon="imp" onClick={handleImport} />
          <Button label="Export" icon="exp" onClick={handleExport} />
        </>
      }
    >
      <div className="row">
        <SegmentedControl
          items={CATEGORIES as unknown as string[]}
          selectedIndex={selectedCategoryIdx}
          onChange={(_, idx) => setSelectedCategoryIdx(idx)}
        />
        <div style={{ flex: 1, minWidth: "180px" }}>
          <Input
            value={searchQuery}
            placeholder="Search presets"
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      </div>
      <div className="g4">
        {filteredPresets.map((p) => (
          <Tile
            key={p.name}
            icon="tag"
            title={p.name}
            subtitle={p.subtitle}
            dataGo="rack"
            href="#rack"
          />
        ))}
      </div>
      {importedNotice && (
        <div style={{ position: "fixed", bottom: 24, right: 24, zIndex: 100 }} className="pill ok">
          {importedNotice}
        </div>
      )}
    </PageLayout>
  );
};
