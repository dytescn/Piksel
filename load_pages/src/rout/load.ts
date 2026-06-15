import { load_tpl } from "../view/load.ts";

export const download_init = () => {
  const node_download = document.getElementById("download-list");
  if (node_download) {
    node_download.innerHTML = load_tpl;
  }
};
