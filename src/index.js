import 'dotenv/config' // NOTE: This will need changing if we split this file.
import Koa from 'koa'
import Router from 'koa-router'
import serve from 'koa-static'

const app = new Koa()

// Serve the main public assets
// TODO: Would be nice to be able to hit a url without the /#/
app.use(serve(process.env.PUBLIC_PATH || 'public'))

// Set up the router for the API
const router = new Router()
router.get('/api', ctx => {
	ctx.body = 'TODO: Like, the entire API.'
})

app.use(router.routes())
app.use(router.allowedMethods())

// Boot the server
app.listen(process.env.PORT || 3001)
