import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as url from 'node:url';
import 'dotenv/config';

const __dirname = new url.URL('.', import.meta.url).pathname;

const cookieJar = Object.fromEntries(
	process.env['HEADER_COOKIE']
		.split(' ')
		.map(cookie => cookie.replace(/;$/, '').split(/=(.*)/s).slice(0, 2)),
);

const myHeaders = new Headers();
myHeaders.append('authorization', process.env['HEADER_AUTHORIZATION']);
myHeaders.append('cache-control', 'no-cache');
myHeaders.append('content-type', 'application/json');
myHeaders.append('cookie', process.env['HEADER_COOKIE']);
myHeaders.append('origin', 'https://www.youtube.com');
myHeaders.append('pragma', 'no-cache');
myHeaders.append(
	'user-agent',
	'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/115.0.0.0 Safari/537.36 Edg/115.0.1901.200',
);
myHeaders.append('x-goog-authuser', '0');
myHeaders.append('x-goog-visitor-id', process.env['HEADER_GOOG_VIS_ID']);
myHeaders.append('x-origin', 'https://www.youtube.com');
myHeaders.append('x-youtube-bootstrap-logged-in', 'true');
myHeaders.append('x-youtube-client-name', '1');
myHeaders.append('x-youtube-client-version', '2.20230817.02.00');

const baseBody = {
	context: {
		client: {
			clientName: 'WEB',
			clientVersion: '2.20230807.06.00',
		},
	},
};

const getReqOpts = token => ({
	method: 'POST',
	headers: myHeaders,
	body: JSON.stringify(
		token !== null
			? { ...baseBody, continuation: token }
			: { ...baseBody, browseId: 'FEhistory' },
	),
	redirect: 'follow',
});

const sleep = x => new Promise(r => setTimeout(r, x));

const parseHeaders = header =>
	header.replace(/;$/, '').split(/=(.*)/s).slice(0, 2);

const fetchData = async () => {
	const MAX_ITERATIONS = 300;
	const SLEEP_MS = 500;
	let token = null;
	// let token =
	// 	'4qmFsgJSEglGRWhpc3RvcnkaLkNKa0ZlaHQ1Y21FMWFIZHZUME5uZDBremNXMDRiRkZaVVhGS00wSXlkMFUlM0SaAhRicm93c2UtZmVlZEZFaGlzdG9yeQ%3D%3D';
	let counter = 0;
	const promises = [];
	do {
		console.log(counter + 1, token);

		const options = getReqOpts(token);

		const res = await fetch(
			'https://www.youtube.com/youtubei/v1/browse?prettyPrint=false',
			options,
		);
		const data = await res.json();

		// console.log(res.headers);
		const cookies = res.headers.getSetCookie();
		const a = Object.fromEntries(
			cookies.map(cookie => parseHeaders(cookie.split(' ').at(0))),
		);
		Object.assign(cookieJar, a);

		// await fs.writeFile(
		// 	path.join(__dirname, `original-${counter}.json`),
		// 	JSON.stringify(data),
		// 	'utf8',
		// );

		await sleep(SLEEP_MS);

		if (token !== null && !data.onResponseReceivedActions) {
			console.log(JSON.stringify(data));
			continue;
		}

		const base =
			token !== null
				? data.onResponseReceivedActions[0].appendContinuationItemsAction
						.continuationItems
				: data.contents.twoColumnBrowseResultsRenderer.tabs[0].tabRenderer
						.content.sectionListRenderer.contents;

		token =
			base.at(-1).continuationItemRenderer.continuationEndpoint
				.continuationCommand.token;

		// const firstTitle = base.at(0).itemSectionRenderer.contents.at(0)
		// 	.videoRenderer.title.runs[0].text; // .at(0).videoRenderer.title.runs[0].text;
		// console.log(firstTitle);

		const x = base
			.map(x => x.itemSectionRenderer)
			.filter(x => x && x.contents)
			.map(x =>
				x.contents.map(content => ({
					parentHeader:
						x.header.itemSectionHeaderRenderer.title.runs?.[0]?.text ??
						x.header.itemSectionHeaderRenderer.title.simpleText,
					content,
				})),
			)
			.flat()
			// Filter out shorts
			.filter(
				x => x.content.reelShelfRenderer?.title?.runs?.[0]?.text === undefined,
			)
			.map(x => ({
				header: x.parentHeader,
				videoId: x.content.videoRenderer.videoId,
				title: x.content.videoRenderer.title.runs[0].text,
			}));

		promises.push(x);
		counter++;
	} while (token !== null && counter < MAX_ITERATIONS);

	return promises.flat();
};

fetchData()
	.then(data =>
		fs.writeFile(
			path.join(__dirname, 'ids.json'),
			JSON.stringify(data),
			'utf8',
		),
	)
	.then(() => fs.readFile(path.join(__dirname, '..', '.env'), 'utf8'))
	.then(contents => {
		const newCookies = Object.entries(cookieJar)
			.map(([k, v]) => `${k}=${v}`)
			.join('; ');

		const newEnvs = contents
			.split('\n')
			.map(x =>
				x.split(/=(.*)/).at(0) === 'HEADER_COOKIE'
					? `HEADER_COOKIE="${newCookies}"`
					: x,
			)
			.join('\n');

		return fs.writeFile(path.join(__dirname, '..', '.env'), newEnvs, 'utf8');
	})
	.then(() => console.log('Done'))
	.catch(console.error);
