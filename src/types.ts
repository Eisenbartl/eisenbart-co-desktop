export interface WorkspaceInfo {
  organizationId: string;
  organizationName: string;
  counselorId: string;
  counselorName: string;
}

export const CLIENT_STATUSES = [
  'New Intake',
  'Information Collection',
  'Financial Review',
  'Planning',
  'Action Plan',
  'Follow-Up',
  'Archived',
] as const;

export type ClientStatus = (typeof CLIENT_STATUSES)[number];

export interface Client {
  id: string;
  organizationId: string;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  householdSize: number;
  status: ClientStatus;
  goals: string;
  createdAt: string;
  updatedAt: string;
}

export interface ClientInput {
  id?: string | null;
  firstName: string;
  lastName: string;
  email: string;
  phone: string;
  householdSize: number;
  status: ClientStatus;
  goals: string;
}

export interface ClientNote {
  id: string;
  clientId: string;
  counselorId: string;
  counselorName: string;
  body: string;
  createdAt: string;
}

export interface SetupInput {
  organizationName: string;
  counselorName: string;
  password: string;
}
