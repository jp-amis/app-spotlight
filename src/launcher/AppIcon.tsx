import { createResource, Show } from "solid-js";
import { appIcon } from "../lib/api";

// Module-level cache so an icon is fetched once per path across re-renders.
const cache = new Map<string, string | null>();

async function loadIcon(path: string): Promise<string | null> {
  if (cache.has(path)) return cache.get(path)!;
  let uri: string | null = null;
  try {
    uri = await appIcon(path);
  } catch {
    uri = null;
  }
  cache.set(path, uri);
  return uri;
}

export default function AppIcon(props: { path: string }) {
  const [icon] = createResource(() => props.path, loadIcon);
  return (
    <Show
      when={icon()}
      fallback={<div class="h-8 w-8 shrink-0 rounded-md bg-black/10 dark:bg-white/10" />}
    >
      <img src={icon()!} alt="" class="h-8 w-8 shrink-0" draggable={false} />
    </Show>
  );
}
