/* 0041d8f2 FUN_0041d8f2 */

void * __thiscall FUN_0041d8f2(void *this,uint param_1)

{
  FUN_0043961a();
  if ((param_1 & 1) != 0) {
    _free(this);
  }
  return this;
}
