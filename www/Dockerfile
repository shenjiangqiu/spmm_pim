#  docker run --name some-nginx -v /some/content:/usr/share/nginx/html:ro -d nginx
From nginx:latest
workdir /usr/share/nginx/html/
COPY ./dist/ /usr/share/nginx/html/
EXPOSE 80
