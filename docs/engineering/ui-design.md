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

Today navigation is **Dashboard**, **Users**, **Assign to device**, **Devices**, and **Credentials**. Enrollment sits inside Credentials: a credential belongs to a user and to the device it was enrolled on. Assign to device picks a device, assigns selected users, and adds those users on the COSEC device. Do not add Audit or Settings until those domains exist.

## Page pattern

Title → short description → primary action → search/filters → main content → supporting info.

One purpose per page. Tables are first-class: headers, row actions, empty/loading/error states, search, status with text+icon+color (never color alone).

## Actions

- Primary: Create / Save / Connect
- Secondary: Cancel / Edit
- Destructive: confirm with what happens, reversibility, and which resource

Activate and deactivate are domain operations (`activateUser`, `deactivateUser`). The update form changes username only. The Devices action assigns a user to many devices locally; it does not create the user on the Matrix device. The UI must not invent a status dropdown or Matrix columns.

## Feedback

- Toasts for short success/failure
- Localized loading (not full-page spinners for table refreshes)
- Safe error copy only — no SQL, stack traces, or secrets

## Security UX

Never show passwords, tokens, device credentials, or biometric data. Frontend validation is not authorization.
