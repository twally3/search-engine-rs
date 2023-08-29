import * as fs from 'node:fs';
import * as path from 'node:path';
import * as url from 'node:url';

const __dirname = new url.URL('.', import.meta.url).pathname;

const fetchTranscripts = videoId =>
	fetch(`https://www.youtube.com/watch?v=${videoId}`)
		.then(r => r.text())
		.then(html =>
			html.includes('action="https://consent.youtube.com/s"')
				? Promise.reject(new Error('Unhandled Consent Cookie'))
				: html,
		)
		.then(html => {
			const splitHtml = html.split('"captions":');

			if (splitHtml.length <= 1) {
				if (videoId.startsWith('http://') || videoId.startsWith('http://'))
					throw new Error('Invalid video ID ' + videoId);
				if (html.search('class="g-recaptcha"') !== -1)
					throw new Error('Too Many Requests');
				if (html.search('"playabilityStatus":') === -1)
					throw new Error('Video unavailable');
				throw new Error('Unhandled captions error');
			}

			const captionsJson = JSON.parse(
				splitHtml[1].split(',"videoDetails')[0].replace('\n', ''),
			)['playerCaptionsTracklistRenderer'];

			if (!captionsJson) throw new Error('Captions disabled!');
			if (!('captionTracks' in captionsJson))
				throw new Error('No transcripts available!');

			const translationLanguages = captionsJson['translationLanguages'].map(
				translationLanguage => ({
					language: translationLanguage['languageName']['simpleText'],
					language_code: translationLanguage['languageCode'],
				}),
			);

			const manuallyCreated = new Map();
			const generated = new Map();

			for (const caption of captionsJson['captionTracks']) {
				const dict = caption['kind'] === 'asr' ? generated : manuallyCreated;
				dict.set(caption['languageCode'], {
					videoId,
					baseUrl: caption['baseUrl'],
					simpleText: caption['name']['simpleText'],
					languageCode: caption['languageCode'],
					isGenerated: caption['kind'] === 'asr',
					translatedLanguages: caption['isTranslatable']
						? translationLanguages
						: [],
				});
			}

			return { manuallyCreated, generated };
		});

const findTranscript = (languageCodes, dicts) => {
	for (const languageCode of languageCodes)
		for (const dict of dicts)
			if (dict.has(languageCode)) return dict.get(languageCode);
	return null;
};

const fetchTranscript = transcript =>
	fetch(transcript.baseUrl, { headers: { 'Accept-Language': 'en-US' } }).then(
		r => r.text(),
	);

const videos = JSON.parse(
	fs.readFileSync(path.join(__dirname, 'ids.json'), 'utf-8'),
);

const manifest = {};
const dir = path.join(__dirname, '..', '..', 'transcripts');
if (!fs.existsSync(dir)) {
	fs.mkdirSync(dir, { recursive: true });
}

const chunkSize = 20;

for (let i = 0; i < videos.length; i += chunkSize) {
	console.log(100 * (i / videos.length));

	const localVideos = videos.slice(i, i + chunkSize);
	const promises = [];
	for (const video of localVideos) {
		const { videoId, title } = video;
		manifest[videoId] = title;

		const p = fetchTranscripts(videoId)
			.then(async ({ manuallyCreated, generated }) => {
				const transcript = findTranscript(['en'], [manuallyCreated, generated]);
				if (transcript === null) throw new Error('Failed to find transcript');

				return fetchTranscript(transcript);
			})
			.then(captions => ({
				videoId,
				title,
				captions,
			}))
			.catch(e => {
				console.error(`SKIPPING ${videoId}`, e);
				return Promise.reject(e);
			});
		promises.push(p);
	}

	const results = await Promise.allSettled(promises);
	await Promise.all(
		results
			.filter(x => x.status === 'fulfilled')
			.map(x =>
				fs.promises.writeFile(
					path.join(dir, `${x.value.videoId}.xml`),
					x.value.captions,
					'utf8',
				),
			),
	);
}

fs.writeFileSync(
	path.join(dir, 'manifest.json'),
	JSON.stringify(manifest),
	'utf8',
);
