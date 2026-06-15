import { banner_tpl } from "../view/banner.ts";

export const banner_init = () => {
  const node_banner = document.getElementById("banner");
  if (node_banner) {
    node_banner.innerHTML = banner_tpl;
  }
};
