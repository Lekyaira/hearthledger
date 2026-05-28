export const userStorageKey = 'hearthledger.user.id';
export const userChangedEvent = 'hearthledger:user-changed';

export function readCurrentUserId() {
	return (
		sessionStorage.getItem(userStorageKey) ??
		sessionStorage.getItem('user_id') ??
		sessionStorage.getItem('userId') ??
		''
	);
}

export function setCurrentUserId(userId: string) {
	sessionStorage.setItem(userStorageKey, userId);
	window.dispatchEvent(new CustomEvent(userChangedEvent));
}

export function clearCurrentUserId() {
	sessionStorage.removeItem(userStorageKey);
	sessionStorage.removeItem('user_id');
	sessionStorage.removeItem('userId');
	window.dispatchEvent(new CustomEvent(userChangedEvent));
}
