import { redirect } from '@sveltejs/kit';
import { fetchMe } from '$lib/api';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch }) => {
	try {
		const me = await fetchMe(fetch);
		return { me };
	} catch (e) {
		if (e && typeof e === 'object' && 'status' in e) throw e;
		throw redirect(302, '/login');
	}
};
