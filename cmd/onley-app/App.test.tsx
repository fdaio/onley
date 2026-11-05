import { expect, open, test } from "nsh/test";

test("cleans the duplicate files in a real window", async () => {
  const app = await open(".", { storage: { language: "en" } });
  await app.waitFor("id:scan-button");
  await app.waitFor("text:one.txt");
  await app.press("id:policy-path");
  await app.press("id:delete-files");
  await app.waitFor("text:DELETE DUPLICATES?");
  await app.press("id:delete-confirm");
  await app.waitFor("id:empty-state");
  expect(app.texts().some((text) => text.includes("Deleted"))).toBe(true);
});
