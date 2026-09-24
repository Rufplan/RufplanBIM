// Rufplan.io account, link and publish actions (ADR-016). Errors land in the store.
import { apply } from "./fileActions";
import { errorMessage, ipc, type CloudStatus, type RufplanLink } from "./ipc";
import { useAppStore } from "./store";

async function cloud(action: () => Promise<CloudStatus>): Promise<CloudStatus | null> {
  const s = useAppStore.getState();
  try {
    const status = await action();
    s.setCloud(status);
    return status;
  } catch (err) {
    s.setError(errorMessage(err));
    return null;
  }
}

/** Fetches the account state (signing in again from the stored token if there is one). */
export const refreshCloud = () => cloud(() => ipc.cloudStatus());
export const signIn = (email: string, password: string) =>
  cloud(() => ipc.cloudSignIn(email, password));
export const signInGoogle = () => cloud(() => ipc.cloudSignInGoogle());
export const signOut = () => cloud(() => ipc.cloudSignOut());

export const linkProject = (link: RufplanLink | null) => apply(() => ipc.linkRufplan(link));

export const projectUrl = (slug: string) => `https://rufplan.io/projects/${slug}`;

/** Publishes the current stage's set; resolves to the Rufplan page and what was sent. */
export async function publish(name: string, deliverable: string) {
  const s = useAppStore.getState();
  try {
    const result = await ipc.publishToRufplan(name, deliverable);
    if (result.state) s.setApp(result.state);
    s.setPrompt(`Published "${name}" to ${result.url}`);
    return result;
  } catch (err) {
    s.setError(errorMessage(err));
    return null;
  }
}
