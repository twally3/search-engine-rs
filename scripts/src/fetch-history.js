import * as fs from 'node:fs/promises';
import * as path from 'node:path';
import * as url from 'node:url';

const __dirname = new url.URL('.', import.meta.url).pathname;

const myHeaders = new Headers();
myHeaders.append('authorization', '');
myHeaders.append('cache-control', 'no-cache');
myHeaders.append('content-type', 'application/json');
myHeaders.append('cookie', '');
myHeaders.append('origin', 'https://www.youtube.com');
myHeaders.append('pragma', 'no-cache');
myHeaders.append(
	'user-agent',
	'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36',
);
myHeaders.append('x-goog-authuser', '0');
myHeaders.append('x-goog-visitor-id', '');
myHeaders.append('x-origin', '');
myHeaders.append('x-youtube-bootstrap-logged-in', 'true');
myHeaders.append('x-youtube-client-name', '1');
myHeaders.append('x-youtube-client-version', '2.20230807.06.00');

const raw = JSON.stringify({
	context: {
		client: {
			clientName: 'WEB',
			clientVersion: '2.20230807.06.00',
		},
	},
	browseId: 'FEhistory',
});

const requestOptions = {
	method: 'POST',
	headers: myHeaders,
	body: raw,
	redirect: 'follow',
};

fetch(
	'https://www.youtube.com/youtubei/v1/browse?prettyPrint=false',
	requestOptions,
)
	.then(response => response.json())
	.then(data =>
		fs.writeFile(
			path.join(__dirname, 'ids.json'),
			JSON.stringify(
				Array.from(
					new Set(
						data.contents.twoColumnBrowseResultsRenderer.tabs[0].tabRenderer.content.sectionListRenderer.contents
							.map(x => x.itemSectionRenderer)
							.filter(Boolean)
							.map(x => x.contents)
							.flat()
							.map(x => ({
								videoId: x.videoRenderer.videoId,
								title: x.videoRenderer.title.runs[0].text,
							})),
					),
				),
			),
		),
	)
	.then(() => {
		console.log('Done');
	})
	.catch(error => console.log('error', error));
