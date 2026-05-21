/* 0041d91b FUN_0041d91b */

void * __thiscall FUN_0041d91b(void *this,uint param_1)

{
  FUN_0043ab65((int)this);
  if ((param_1 & 1) != 0) {
    _free(this);
  }
  return this;
}
