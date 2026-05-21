/* 0041338b FUN_0041338b */

void * __thiscall FUN_0041338b(void *this,uint param_1)

{
  if ((param_1 & 2) == 0) {
    FUN_0043961a();
    if ((param_1 & 1) != 0) {
      _free(this);
    }
  }
  else {
    _eh_vector_destructor_iterator_(this,0xfc,*(int *)((int)this + -4),FUN_0043961a);
    if ((param_1 & 1) != 0) {
      _free((void *)((int)this + -4));
    }
    this = (void *)((int)this + -4);
  }
  return this;
}
