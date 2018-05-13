import 'dotenv/config'
import Koa from 'koa'

const app = new Koa()

app.use(ctx => {
	ctx.body = 'This was a triumph!'
})

app.listen(process.env.PORT || 3001)
