import sys
from html.parser import HTMLParser
class Reader(HTMLParser):
 def __init__(self): super().__init__(); self.active=False
 def handle_starttag(self,t,a):
  if t=='section' and ('id','main-content') in a: self.active=True
  if self.active and t in ['p','h1','h2','h3','h4','pre','section','tr','details']: print()
 def handle_data(self,d):
  if self.active: print(d,end='')
for p in sys.argv[1:]:
 print('\nPAGE:',p); Reader().feed(open('/kit/doc/pert_ui/'+p).read())
