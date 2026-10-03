"use client";

import { useState, type FormEvent } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/button";
import { Notice } from "@/components/notice";

export function AdminLoginForm() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(event: FormEvent) {
    event.preventDefault();
    setError(null);

    if (!email.trim() || !password) {
      setError("Email and password are required.");
      return;
    }

    setSubmitting(true);
    const response = await fetch("/api/admin/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email, password }),
    });
    setSubmitting(false);

    if (response.ok) {
      router.push("/admin");
      router.refresh();
      return;
    }

    setError("Invalid email or password.");
  }

  return (
    <form onSubmit={handleSubmit} noValidate className="mx-auto flex max-w-[400px] flex-col gap-5 py-16">
      <h1 className="m-0 text-2xl font-semibold">Admin Login</h1>
      <div>
        <label htmlFor="admin-email" className="mb-1.5 block text-sm font-semibold">Email</label>
        <input
          id="admin-email"
          type="email"
          className="field"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          autoComplete="username"
        />
      </div>
      <div>
        <label htmlFor="admin-password" className="mb-1.5 block text-sm font-semibold">Password</label>
        <input
          id="admin-password"
          type="password"
          className="field"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
        />
      </div>
      {error && <Notice>{error}</Notice>}
      <Button type="submit" disabled={submitting}>
        {submitting ? "Logging in…" : "Log In"}
      </Button>
    </form>
  );
}
