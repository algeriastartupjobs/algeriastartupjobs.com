import { BrowserWindow, app } from "electron";
import express from "express";
import { getSelectorJS } from "../adapter/selector";
import { matchSourceURL } from "../adapter/matcher";
import { normalizeSourceURL } from "../adapter/normalizer";

const server = express();

server.get("/scrape", async (req, res) => {
  console.log("Scraping...");

  const urlString = req.query.url.toString();

  console.log("Matching source URL...");
  const source = matchSourceURL(urlString);
  if (!source) {
    console.log("Source URL not matched");
    res.sendStatus(400);
    return;
  }

  console.log(`Normalizing ${source} URL...`);
  const normalizedURL = normalizeSourceURL(source, urlString);

  console.log("Opening scraping window...");
  const win = new BrowserWindow({ show: false });

  console.log("Clearing scraping window cache...");
  await win.webContents.session.clearStorageData();
  await win.webContents.session.clearCache();

  console.log("Loading scraping window...");
  win.loadURL(normalizedURL);
  win.webContents.on("did-finish-load", () => {
    win.webContents
      .executeJavaScript(getSelectorJS(source))
      .then((data) => {
        let description = data[1] || "";
        description = description.replace(/(\r\n|\n|\r){2,}/gm, "\n\n");

        res.json({
          title: data[0] || "",
          description,
          poster: data[2] || "",
        });
        console.log("Scraping done");
      })
      .catch((err) => {
        console.log("Scraping error", err);
        res.sendStatus(500);
      })
      .finally(() => {
        win.close();
        console.log("Scraping window closed");
      });
  });
});

server.listen(8383, () => {
  console.log("Server listening on port 8383");
});

app.on("window-all-closed", (e: Event) => {
  e.preventDefault();
});
