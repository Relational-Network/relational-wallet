"use client";

import { useAuth } from "@clerk/nextjs";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef } from "react";

const RESYNC_RELOAD_KEY = "wallet-web:clerk-session-resync";

function currentRedirectUrl(pathname: string): string {
  if (typeof window === "undefined") {
    return pathname;
  }
  return `${window.location.origin}${pathname}${window.location.search}`;
}

export function ProtectedSessionResync() {
  const { isLoaded, isSignedIn, getToken } = useAuth();
  const pathname = usePathname();
  const router = useRouter();
  const resyncStartedRef = useRef(false);

  useEffect(() => {
    if (!isLoaded || isSignedIn !== false) return;
    router.replace(`/sign-in?redirect_url=${encodeURIComponent(currentRedirectUrl(pathname))}`);
  }, [isLoaded, isSignedIn, pathname, router]);

  useEffect(() => {
    if (!isLoaded || !isSignedIn || resyncStartedRef.current) return;
    resyncStartedRef.current = true;

    let cancelled = false;

    const resyncSession = async () => {
      try {
        const [templateToken, sessionToken] = await Promise.all([
          getToken({ template: "default", skipCache: true }).catch(() => null),
          getToken({ skipCache: true }).catch(() => null),
        ]);

        if (cancelled) return;

        if (templateToken ?? sessionToken) {
          if (typeof window !== "undefined") {
            window.sessionStorage.removeItem(RESYNC_RELOAD_KEY);
          }
          return;
        }
      } catch {
        if (cancelled) return;
      }

      if (typeof window === "undefined") return;

      const priorAttempt = window.sessionStorage.getItem(RESYNC_RELOAD_KEY);
      if (priorAttempt !== pathname) {
        window.sessionStorage.setItem(RESYNC_RELOAD_KEY, pathname);
        window.location.reload();
        return;
      }

      window.sessionStorage.removeItem(RESYNC_RELOAD_KEY);
      router.replace(`/sign-in?redirect_url=${encodeURIComponent(currentRedirectUrl(pathname))}`);
    };

    void resyncSession();

    return () => {
      cancelled = true;
    };
  }, [getToken, isLoaded, isSignedIn, pathname, router]);

  return null;
}
