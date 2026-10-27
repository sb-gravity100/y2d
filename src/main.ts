import { mount } from "svelte";
import App from "./App.svelte";
import Installer from "./Installer.svelte";
import "./app.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("The app root element is missing");
}

console.info("[y2d] mounting Svelte application");
mount(new URLSearchParams(location.search).has("installer") ? Installer : App, { target });
