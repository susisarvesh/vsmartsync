import { invoke } from "@tauri-apps/api/core";
import type {
  CreateDeviceInput,
  Device,
  UpdateDeviceInput,
} from "../types/devices";

/**
 * Frontend access to Rust must go through this services layer.
 * Do not call PostgreSQL or Matrix devices from React.
 * Never request or display device passwords or ciphertext.
 */
export async function listDevices(): Promise<Device[]> {
  return invoke<Device[]>("list_devices");
}

export async function createDevice(input: CreateDeviceInput): Promise<Device> {
  return invoke<Device>("create_device", {
    deviceName: input.deviceName,
    host: input.host,
    port: input.port,
    macAddress: blankToNull(input.macAddress),
    username: input.username,
    password: input.password,
  });
}

export async function updateDevice(input: UpdateDeviceInput): Promise<Device> {
  return invoke<Device>("update_device", {
    id: input.id,
    deviceName: input.deviceName,
    host: input.host,
    port: input.port,
    macAddress: blankToNull(input.macAddress),
    username: input.username,
  });
}

export async function activateDevice(id: string): Promise<Device> {
  return invoke<Device>("activate_device", { id });
}

export async function deactivateDevice(id: string): Promise<Device> {
  return invoke<Device>("deactivate_device", { id });
}

function blankToNull(value: string): string | null {
  const trimmed = value.trim();
  return trimmed.length === 0 ? null : trimmed;
}

export async function setDevicePassword(
  id: string,
  password: string,
): Promise<Device> {
  return invoke<Device>("set_device_password", { id, password });
}

export async function testDeviceConnection(id: string): Promise<Device> {
  return invoke<Device>("test_device_connection", { id });
}

export function deviceErrorMessage(code: string): string {
  switch (code) {
    case "DEVICE_INVALID_NAME":
      return "Enter a device name (1–200 characters).";
    case "DEVICE_INVALID_MAC":
      return "Enter a MAC address as six hex pairs, or leave it blank.";
    case "DEVICE_INVALID_HOST":
      return "Enter a hostname or IPv4 address only (no URL).";
    case "DEVICE_INVALID_PORT":
      return "Port must be between 1 and 65535.";
    case "DEVICE_INVALID_USERNAME":
      return "Enter a device username.";
    case "DEVICE_INVALID_PASSWORD":
      return "Enter a device password.";
    case "DEVICE_DUPLICATE":
      return "A device with that host and port already exists.";
    case "DEVICE_NOT_FOUND":
      return "That device does not exist.";
    case "DEVICE_OFFLINE":
      return "Could not reach the device.";
    case "DEVICE_AUTH_FAILED":
      return "Device rejected the username or password.";
    case "DEVICE_BAD_RESPONSE":
      return "The device returned an unexpected response.";
    case "DEVICE_SECRET_UNAVAILABLE":
      return "Could not access the secure credential store.";
    case "DEVICE_SECRET_CORRUPT":
      return "The saved device password cannot be read. Choose Set Password, enter it again, then test the connection.";
    case "DATABASE_UNAVAILABLE":
      return "PostgreSQL is not connected.";
    default:
      return "Could not complete that device action.";
  }
}
