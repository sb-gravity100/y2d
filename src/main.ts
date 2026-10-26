import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

const target = document.getElementById("app");
if (!target) {
  throw new Error("The app root element is missing");
}

console.info("[y2d] mounting Svelte application");
mount(App, { target });
