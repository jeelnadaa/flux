import React from "react";
import {
  PageLayout,
  Section,
  Button,
  SegmentedControl,
  Chip,
  Pill,
  Toggle,
  Slider,
  Field,
  Input,
  Select,
  Callout,
  EffectCard,
  ProgressBar,
  CanvasViz,
} from "@flux/ui";

export const KitScreen: React.FC = () => {
  return (
    <PageLayout
      currentTab="home"
      title="The kit."
      subtitle="Every part of Flux. A page that has no design yet gets built from these, and nothing else."
    >
      <Section title="Type">
        <div className="bigh" style={{ fontSize: "clamp(44px,8cqw,110px)" }}>
          Display 900
        </div>
        <div className="h1">Page title</div>
        <div className="h2">Section title</div>
        <div className="h3">Group title</div>
        <p className="lede" style={{ margin: 0 }}>
          Body copy, weight 500. Short sentences, in plain words.
        </p>
        <span className="lab">Meta label, Geist Mono 400. Units, states and small labels.</span>
      </Section>

      <Section title="Buttons">
        <div className="row">
          <Button variant="primary" label="Primary" />
          <Button label="Secondary" />
          <Button variant="ghost" label="Ghost" />
          <Button variant="magic" icon="mag" label="Magic" />
          <Button variant="danger" label="Delete" />
          <Button variant="primary" large label="Large" />
          <Button icon="plus" ariaLabel="Add" />
        </div>
      </Section>

      <Section title="Choices">
        <div className="row">
          <SegmentedControl items={["One", "Two", "Three"]} selectedIndex={0} />
          <Chip label="Chip" active />
          <Chip label="Chip" />
          <Pill label="Label" />
          <Pill label="Good" variant="ok" />
          <Pill label="Check" variant="warn" />
          <Pill label="Failed" variant="bad" />
          <Pill label="Selected" variant="ai" />
        </div>
        <div className="g2">
          <Toggle label="Toggle on" defaultChecked sub="With a line of help" />
          <Toggle label="Toggle off" defaultChecked={false} />
        </div>
      </Section>

      <Section title="Sliders">
        <Slider label="Main control" defaultValue={62} valueText="62%" big />
        <div className="g3">
          <Slider label="Threshold" defaultValue={62} valueText="-18 dB" />
          <Slider label="Ratio" defaultValue={40} valueText="3.2:1" />
          <Slider label="Mix" defaultValue={100} valueText="100%" />
        </div>
      </Section>

      <Section title="Fields">
        <div className="g3">
          <Field label="Text">
            <Input defaultValue="Harbor Lights" />
          </Field>
          <Field label="Select">
            <Select options={["48 kHz", "44.1 kHz"]} selectedIndex={0} />
          </Field>
          <Field label="Empty">
            <Input placeholder="Placeholder" />
          </Field>
        </div>
      </Section>

      <Section title="Blocks">
        <div className="g3">
          <div className="box">
            <b className="h3">Box</b>
            <p className="dim">The normal panel.</p>
          </div>
          <div className="box inv">
            <div className="stat">-14.2</div>
            <span className="lab">Inverse block, for key numbers</span>
          </div>
          <div className="box pal-nightcore">
            <b className="h3">Palette block</b>
            <p>Any theme's colors, on one block.</p>
          </div>
        </div>
        <Callout icon="info">A note.</Callout>
        <Callout icon="warn" variant="warn">
          A warning that needs attention.
        </Callout>
        <Callout icon="warn" variant="bad">
          An error, always with a way forward.
        </Callout>
        <Callout icon="lock" variant="ac">
          A key promise, in the inverse color.
        </Callout>
      </Section>

      <Section title="Effect cards">
        <div className="g4">
          <EffectCard
            palette="cassette"
            effect="tape"
            word="Cassette"
            leftMeta="Cassette"
            rightMeta="Wow"
            dataGo="kit"
          />
          <EffectCard
            palette="underwater"
            effect="water"
            word="Under"
            leftMeta="Underwater"
            rightMeta="12 m"
            dataGo="kit"
          />
          <EffectCard
            palette="tapestop"
            effect="stop"
            word="Stop"
            leftMeta="Tape stop"
            rightMeta="1.4 s"
            dataGo="kit"
          />
          <EffectCard
            palette="nightcore"
            effect="night"
            word="Night"
            leftMeta="Nightcore"
            rightMeta="1.25×"
            dataGo="kit"
          />
        </div>
      </Section>

      <Section title="Data">
        <div className="g2">
          <CanvasViz type="wave" seed={3} stereo step={2} height={120} />
          <CanvasViz type="spec" seed={3} height={120} />
        </div>
        <ProgressBar percent={62} />
        <ProgressBar percent={40} running />
        <div className="tw">
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Size</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td className="b7">Table row</td>
                <td className="mono">412 MB</td>
              </tr>
              <tr>
                <td className="b7">Table row</td>
                <td className="mono">38 MB</td>
              </tr>
            </tbody>
          </table>
        </div>
      </Section>

      <Section title="Track colors">
        <div className="row">
          {[1, 2, 3, 4, 5, 6, 7, 8].map((n) => (
            <span
              key={n}
              className="pill ai"
              style={{
                background: `var(--t${n})`,
                color: `var(--t${n}-on)`,
              }}
            >
              Track {n}
            </span>
          ))}
        </div>
      </Section>
    </PageLayout>
  );
};
