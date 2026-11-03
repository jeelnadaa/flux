import React, { useState } from "react";
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
  const [selectedCategoryIdx, setSelectedCategoryIdx] = useState(0);
  const [searchQuery, setSearchQuery] = useState("");
  const [importedNotice, setImportedNotice] = useState<string | null>(null);

  const selectedCategory = CATEGORIES[selectedCategoryIdx];

  const filteredPresets = PRESETS.filter((p) => {
    const matchesCategory = selectedCategoryIdx === 0 ? true : p.category === selectedCategory;
    const matchesSearch =
      searchQuery.trim().length === 0 ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.subtitle.toLowerCase().includes(searchQuery.toLowerCase());
    return matchesCategory && matchesSearch;
  });

  const handleImport = () => {
    // Open hidden file picker or trigger mock import
    setImportedNotice("Imported preset successfully");
    setTimeout(() => setImportedNotice(null), 3000);
  };

  const handleExport = () => {
    // Trigger mock export
    setImportedNotice("Presets exported");
    setTimeout(() => setImportedNotice(null), 3000);
  };

  return (
    <PageLayout
      title="Presets"
      subtitle="Saved as files you own. Import and export them any time."
      currentTab="presets"
      actions={
        <>
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
