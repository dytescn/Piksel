import { main_tpl } from "../view/main.ts";
import { lang_init } from "./lang.ts";

export const main_init = () => {
  const node_app = document.getElementById("app");
  if (node_app) {
    node_app.innerHTML = main_tpl;
  }
  lang_init();
};
