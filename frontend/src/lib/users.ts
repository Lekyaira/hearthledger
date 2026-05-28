export type UserRole = 'member' | 'admin';

export type User = {
	id: string;
	name: string;
	role: UserRole;
};
