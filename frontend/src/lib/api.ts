import {
	check,
	cliLogin,
	countImages,
	deleteImage,
	getImage,
	listImages,
	login,
	renameImage,
	restoreImage,
	upload,
	type AuthCheckResponse,
	type AuthSuccessResponse,
	type ImageCountResponse,
	type ImageDetailResponse,
	type ImageListResponse,
	type UploadRequest,
	type UploadResponse
} from '$lib/sdk';
import { client } from '$lib/sdk/client.gen';

export type FetchLike = typeof fetch;

export type ImageListParams = {
	page?: number;
	perPage?: number;
	name?: string;
	dateFrom?: string;
	dateTo?: string;
	deleted?: boolean;
};

function configureClient(fetchImpl?: FetchLike) {
	client.setConfig({
		baseUrl: '',
		credentials: 'include',
		fetch: fetchImpl,
		responseStyle: 'fields'
	});
	return client;
}

function unwrap<T>(result: { data?: T; error?: unknown; response: Response }): T {
	if (result.response.ok && result.data !== undefined) {
		return result.data;
	}

	const message =
		typeof result.error === 'string'
			? result.error
			: result.response.statusText || 'Request failed';
	throw new Error(message);
}

export async function loginWithToken(
	token: string,
	fetchImpl?: FetchLike
): Promise<AuthSuccessResponse> {
	const result = await cliLogin({
		client: configureClient(fetchImpl),
		query: { token }
	});
	return unwrap(result);
}

export async function loginWithCredentials(
	input: string,
	fetchImpl?: FetchLike
): Promise<AuthSuccessResponse> {
	const result = await login({
		client: configureClient(fetchImpl),
		body: { token: input.trim() }
	});
	return unwrap(result);
}

export async function fetchMe(fetchImpl?: FetchLike): Promise<AuthCheckResponse> {
	const result = await check({ client: configureClient(fetchImpl) });
	return unwrap(result);
}

export async function fetchImages(
	params: ImageListParams = {},
	fetchImpl?: FetchLike
): Promise<ImageListResponse> {
	const result = await listImages({
		client: configureClient(fetchImpl),
		query: {
			page: params.page,
			per_page: params.perPage,
			name: params.name?.trim() || undefined,
			date_from: params.dateFrom || undefined,
			date_to: params.dateTo || undefined,
			deleted: params.deleted
		}
	});
	return unwrap(result);
}

export async function fetchImageCounts(fetchImpl?: FetchLike): Promise<ImageCountResponse> {
	const result = await countImages({ client: configureClient(fetchImpl) });
	return unwrap(result);
}

export async function renameManagedImage(
	id: number,
	displayName: string,
	fetchImpl?: FetchLike
): Promise<void> {
	const result = await renameImage({
		client: configureClient(fetchImpl),
		path: { id },
		body: { display_name: displayName.trim() }
	});
	unwrap(result);
}

export async function deleteManagedImage(id: number, fetchImpl?: FetchLike): Promise<void> {
	const result = await deleteImage({
		client: configureClient(fetchImpl),
		path: { id }
	});
	unwrap(result);
}

export async function restoreManagedImage(id: number, fetchImpl?: FetchLike): Promise<void> {
	const result = await restoreImage({
		client: configureClient(fetchImpl),
		path: { id }
	});
	unwrap(result);
}

export async function uploadImage(
	file: File,
	keepMetadataFields: string[] | null = null,
	fetchImpl?: FetchLike
): Promise<UploadResponse> {
	const body: UploadRequest = {
		file,
		keep_metadata_fields: keepMetadataFields === null ? null : keepMetadataFields.join(',')
	};
	const result = await upload({
		client: configureClient(fetchImpl),
		body
	});
	return unwrap(result);
}

export async function logoutUser(): Promise<void> {
	// No backend logout endpoint; session expires naturally.
	// Caller should navigate to /login after this.
}

export async function fetchImageDetail(
	id: number,
	fetchImpl?: FetchLike
): Promise<ImageDetailResponse> {
	const result = await getImage({
		client: configureClient(fetchImpl),
		path: { id }
	});
	return unwrap(result);
}
