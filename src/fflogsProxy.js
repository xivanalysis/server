import axios from 'axios'
import Koa from 'koa'
import {promisify} from 'util'
import {gzip, gunzip} from 'zlib'

import redis from './redis'

const asyncGunzip = promisify(gunzip)

const proxy = new Koa()

// FFLogs proxy
const fflogsApi = axios.create({
	baseURL: 'https://www.fflogs.com/v1/',
})

// 7 days (in seconds)
// eslint-disable-next-line no-magic-numbers
const PROXY_CACHE_EXPIRY = 60 * 60 * 24 * 7

proxy.use(async ctx => {
	// Default to our api key, but allow overrides
	let apiKey = process.env.FFLOGS_API_KEY
	const query = ctx.query
	if (query.api_key) {
		apiKey = query.api_key
		delete query.api_key
	}

	// Allow bypassing the cache w/ special param
	// TODO: Limit to users w/ their own keys?
	const bypassCache = query.bypassCache || false
	delete query.bypassCache

	// Make sure any changes we've made to the query are taken into account
	ctx.query = query

	// Build the URL that we'll be requesting (and using as a key!)
	const url = ctx.url
	const key = 'proxy:fflogs:' + url

	if (bypassCache !== 'true') {
		// Check if we have a cached copy, return that if we do
		const cached = await redis.getBuffer(key)
		if (cached) {
			// TODO: We're basically gunzipping it so koa can gzip it again for us
			//       Look into pissing about with it so that's not nessecary.
			const respBuf = await asyncGunzip(cached)
			ctx.body = respBuf.toString()
			return
		}
	}

	// No cached copy, grab it down
	let response = null
	try {
		response = await fflogsApi.get(url, {
			params: {api_key: apiKey},
		})
	} catch (e) {
		// Forwarding the error to the client, bypassing cache
		ctx.status = e.response.status
		ctx.body = e.response.data
		return
	}

	// Set the response
	const {data} = response
	ctx.body = data

	// If data is a string, it's a relatively safe bet that upstream has an error. Avoid cachine and send a 500.
	if (typeof data === 'string') {
		ctx.status = 500
		return
	}

	// Save it to the cache in the background (don't need to wait)
	gzip(JSON.stringify(data), (err, buf) => {
		if (err) { throw err }
		redis.set(key, buf, 'EX', PROXY_CACHE_EXPIRY)
	})
})

export default proxy
