# UI design guidelines

**Status:** Binding for React UI work. Light theme only.

Vsmart Sync is an **enterprise desktop** application. Prioritize clarity, information density, consistency, accessibility, and predictability. Do not design a marketing site, consumer mobile app, crypto dashboard, or decorative startup UI.

## Stack

- React + TypeScript
- Tailwind CSS (light theme)
- shadcn/ui-style primitives under `src/components/ui/`
- Lucide icons
- TanStack Query for async/application data
- React Hook Form + Zod for forms (frontend validation is UX only; Rust remains the security boundary)

Do not add another component library or a second visual system. Do not introduce a dark theme.

## Shell

```
Top bar
Sidebar (only modules that exist) | Main content
```

Today navigation is **Dashboard**, **Users**, and **Devices**. Do not add Enrollment, Sync, Audit, or Settings until those domains exist.

## Page pattern

Title → short description → primary action → search/filters → main content → supporting info.

One purpose per page. Tables are first-class: headers, row actions, empty/loading/error states, search, status with text+icon+color (never color alone).

## Actions

- Primary: Create / Save / Connect
- Secondary: Cancel / Edit
- Destructive: confirm with what happens, reversibility, and which resource

Deactivate is a domain operation (`deactivateUser`). The UI must not invent status fields or Matrix columns.

## Feedback

- Toasts for short success/failure
- Localized loading (not full-page spinners for table refreshes)
- Safe error copy only — no SQL, stack traces, or secrets

## Security UX

Never show passwords, tokens, device credentials, or biometric data. Frontend validation is not authorization.
