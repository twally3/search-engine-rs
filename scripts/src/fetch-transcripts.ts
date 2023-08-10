import { XMLParser } from 'fast-xml-parser';
import * as fs from 'fs';
import * as path from 'path';
import * as he from 'he';

class YouTubeTranscriptApi {
	async listTranscripts(videoId: string) {
		const x = await new TranscriptListFetcher().fetch(videoId);
		const y = x.findTranscript(['en']) as Transcript;
		const z = await y.fetch();
		const parser = new TranscriptParser(false);
		return parser.parse(z);
	}
}

class TranscriptListFetcher {
	async fetch(videoId: string) {
		const x = this.extractCaptionsJson(
			await this.fetchVideoHtml(videoId),
			videoId,
		);

		return TranscriptList.build(videoId, x);
	}

	private extractCaptionsJson(html: string, videoId: string) {
		const splitHtml = html.split('"captions":');

		if (splitHtml.length <= 1) throw new Error('Unhandled captions error');

		const captionsJson = JSON.parse(
			splitHtml[1].split(',"videoDetails')[0].replace('\n', ''),
		)['playerCaptionsTracklistRenderer'];

		if (!captionsJson) throw new Error('Captions disabled!');
		if (!('captionTracks' in captionsJson))
			throw new Error('No transcripts available!');

		return captionsJson;
	}

	private async fetchVideoHtml(videoId: string) {
		const html = await this.fetchHtml(videoId);
		if (html.includes('action="https://consent.youtube.com/s"'))
			throw new Error('Unhandled Consent Cookie');
		return html;
	}

	private fetchHtml(videoId: string) {
		return fetch(`https://www.youtube.com/watch?v=${videoId}`, {
			headers: { 'Accept-Language': 'en-US' },
		}).then(data => data.text());
	}
}

class TranscriptList {
	constructor(
		private videoId: string,
		private manuallyCreated: Map<string, any>,
		private generated: Map<string, any>,
		private translationLanguages: any,
	) {}
	static build(videoId: string, captionsJson: any) {
		const translationLanguages = captionsJson['translationLanguages'].map(
			translationLanguage => ({
				language: translationLanguage['languageName']['simpleText'],
				language_code: translationLanguage['languageCode'],
			}),
		);

		const manuallyCreated = new Map<string, any>();
		const generated = new Map<string, any>();

		for (const caption of captionsJson['captionTracks']) {
			const dict = caption['kind'] === 'asr' ? generated : manuallyCreated;
			dict.set(
				caption['languageCode'],
				new Transcript(
					videoId,
					caption['baseUrl'],
					caption['name']['simpleText'],
					caption['languageCode'],
					caption['kind'] === 'asr',
					caption['isTranslatable'] ? translationLanguages : [],
				),
			);
		}

		return new TranscriptList(
			videoId,
			manuallyCreated,
			generated,
			translationLanguages,
		);
	}

	findTranscript(languageCodes: string[]) {
		for (const languageCode of languageCodes) {
			for (const dict of [this.generated, this.manuallyCreated]) {
				if (dict.has(languageCode)) {
					return dict.get(languageCode);
				}
			}
		}
		throw new Error('Nothing found');
	}
}

class Transcript {
	constructor(
		private videoId: string,
		private url: string,
		private language: string,
		private languageCode: string,
		private isGenerated: boolean,
		private translatedLanguages: unknown[], // self._http_client = http_client // self.video_id = video_id // self._url = url // self.language = language // self.language_code = language_code // self.is_generated = is_generated // self.translation_languages = translation_languages // self._translation_languages_dict = { // 		translation_language['language_code']: translation_language['language'] // 		for translation_language in translation_languages
	) {}

	fetch() {
		return fetch(this.url, {
			headers: {
				'Accept-Language': 'en-US',
			},
		}).then(r => r.text());
	}
}

class TranscriptParser {
	private readonly FORMATTING_TAGS = [
		'strong',
		'em',
		'b',
		'i',
		'mark',
		'small',
		'del',
		'ins',
		'sub',
		'sup',
	];
	private htmlRegex: RegExp;

	constructor(preserveFormatting = false) {
		this.htmlRegex = this.getHtmlRegex(preserveFormatting);
	}

	private getHtmlRegex(preserveFormatting: boolean) {
		if (preserveFormatting) {
			const a = this.FORMATTING_TAGS.join('|');
			const b = new RegExp(`</?(?!/?(${a})\b).*?\b>`, 'i');
			return b;
		} else {
			return /<[^>]*>/i;
		}
	}

	parse(plainData: string) {
		const parser = new XMLParser({ ignoreAttributes: false });
		const data = parser.parse(plainData);
		return data['transcript']['text'].map(
			({ '#text': text, '@_start': start, '@_dur': dur }, i) => {
				const y = data;
				const z = plainData;
				console.log(text);
				const x = {
					text: he.decode(`${text}`.replace(this.htmlRegex, '')),
					start,
					dur,
				};
				return x;
			},
		);
	}
}

// const VIDEO_ID = 'Jsmt4uaL1O8';
// // const VIDEO_ID = '48H5nMQ_8Yg';
// new YouTubeTranscriptApi()
// 	.listTranscripts(VIDEO_ID)
// 	.then(data => {
// 		console.log(data);
// 	})
// 	.catch(console.error);

const ids = JSON.parse(
	fs.readFileSync(path.join(__dirname, 'ids.json'), 'utf-8'),
).slice(0, 10);
console.log(ids);

const main = async () => {
	for (const id of ids) {
		const t = await new YouTubeTranscriptApi().listTranscripts(id);
		fs.writeFileSync(
			path.join(__dirname, '..', 'transcripts', `${id}.txt`),
			t.map(x => x.text).join('\n'),
		);
	}
};

main().then(console.log).catch(console.error);
