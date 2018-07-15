import axios from 'axios'
import Koa from 'koa'

import redis from './redis'

const proxy = new Koa()

// FFLogs proxy
const fflogsApi = axios.create({
	baseURL: 'https://www.fflogs.com/v1/'
})

// 7 days (in seconds)
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
		const cached = await redis.get(key)
		if (cached) {
			ctx.body = cached
			return
		}
	}

	// No cached copy, grab it down
	let response = null
	try {
		response = await fflogsApi.get(url, {
			params: { api_key: apiKey }
		})
	} catch (e) {
		// Forwarding the error to the client, bypassing cache
		ctx.status = e.response.status
		ctx.body = e.response.data
		return
	}

	// Save and respond
	const data = response.data
	redis.set(key, JSON.stringify(data), 'EX', PROXY_CACHE_EXPIRY)
	ctx.body = data
})

export default proxy
