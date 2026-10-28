import { Panel } from "./panel/Panel.tsx";
import { Widget } from "./widget/Widget.tsx";

export function App({ surface }: { surface: "widget" | "panel" }) {
	if (surface === "panel") return <Panel />;
	return <Widget />;
}
