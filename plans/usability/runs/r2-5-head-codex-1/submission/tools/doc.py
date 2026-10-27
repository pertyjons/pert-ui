import sys
from html.parser import HTMLParser
class P(HTMLParser):
 def __init__(self): super().__init__(); self.on=False
 def handle_starttag(self,t,a):
  if t=='section' and ('id','main-content') in a:self.on=True
  if self.on and t in ['p','div','h2','h3','h4','pre','li','tr']:print()
 def handle_data(self,d):
  if self.on:print(d,end='')
for f in sys.argv[1:]:
 print('\nFILE:',f);P().feed(open('/kit/doc/'+f).read())
