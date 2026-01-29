// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

/**
 * API Proxy Route Handler
 *
 * This catch-all route proxies requests from the browser to the backend server.
 * This is necessary because:
 * 1. The backend uses RA-TLS with a self-signed certificate
 * 2. Browsers reject self-signed certificates
 * 3. Server-side Node.js can skip certificate validation (for development)
 *
 * In production, the backend should use a proper CA-signed certificate,
 * or the frontend should be deployed in a way that trusts the RA-TLS CA.
 */

import { NextRequest, NextResponse } from "next/server";
import { auth } from "@clerk/nextjs/server";

const BACKEND_URL = process.env.WALLET_API_BASE_URL || "https://localhost:8080";

/**
 * Proxy a request to the backend.
 */
async function proxyRequest(
  request: NextRequest,
  method: string
): Promise<NextResponse> {
  // Get the path from the URL (everything after /api/proxy/)
  const url = new URL(request.url);
  const pathSegments = url.pathname.replace("/api/proxy", "");
  const backendUrl = `${BACKEND_URL}${pathSegments}${url.search}`;

  // Get auth token from Clerk (server-side)
  const { getToken } = await auth();
  const token = await getToken();

  // Build headers
  const headers: HeadersInit = {
    "Content-Type": "application/json",
  };

  if (token) {
    headers["Authorization"] = `Bearer ${token}`;
  }

  // Forward any x-request-id header
  const requestId = request.headers.get("x-request-id");
  if (requestId) {
    headers["x-request-id"] = requestId;
  }

  try {
    // Build fetch options
    const fetchOptions: RequestInit = {
      method,
      headers,
    };

    // Forward body for POST/PUT/PATCH
    if (["POST", "PUT", "PATCH"].includes(method)) {
      const body = await request.text();
      if (body) {
        fetchOptions.body = body;
      }
    }

    // Make the request to the backend
    const response = await fetch(backendUrl, fetchOptions);

    // Get response body
    const responseText = await response.text();

    // Return the response with appropriate status and headers
    return new NextResponse(responseText || null, {
      status: response.status,
      headers: {
        "Content-Type": response.headers.get("Content-Type") || "application/json",
        // Forward request ID if present
        ...(response.headers.get("x-request-id")
          ? { "x-request-id": response.headers.get("x-request-id")! }
          : {}),
      },
    });
  } catch (error) {
    console.error("[API Proxy] Request failed:", error);

    // Return a meaningful error
    return NextResponse.json(
      {
        error: "Backend request failed",
        message: error instanceof Error ? error.message : "Unknown error",
      },
      { status: 502 }
    );
  }
}

// HTTP method handlers
export async function GET(request: NextRequest) {
  return proxyRequest(request, "GET");
}

export async function POST(request: NextRequest) {
  return proxyRequest(request, "POST");
}

export async function PUT(request: NextRequest) {
  return proxyRequest(request, "PUT");
}

export async function PATCH(request: NextRequest) {
  return proxyRequest(request, "PATCH");
}

export async function DELETE(request: NextRequest) {
  return proxyRequest(request, "DELETE");
}
