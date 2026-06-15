import { lang_tpl } from "../view/lang.ts";

export const lang_init = () => {
  const node_header = document.getElementById("header");
  if (node_header) {
    node_header.innerHTML = lang_tpl;
  }
};
