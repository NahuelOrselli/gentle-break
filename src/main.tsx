import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App.tsx";
import { isTauri } from "./platform.ts";
import "./styles.css";

const surface = new URLSearchParams(window.location.search).get("surface") === "panel" ? "panel" : "widget";
document.documentElement.dataset.surface = surface;
document.documentElement.dataset.preview = isTauri() ? "0" : "1";

const root = document.getElementById("root");
if (!root) throw new Error("root missing");
createRoot(root).render(
	<StrictMode>
		<App surface={surface} />
	</StrictMode>,
);
